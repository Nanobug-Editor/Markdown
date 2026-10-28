//! Exercise the packaged WASM through the public JSON/WIT boundary, without constructing private helper state.

use plugin_protocol::{api, ui};
use std::{collections::BTreeMap, io::Read, sync::OnceLock};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, HasSelf, Linker},
};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxView, WasiView};

// The interface is frozen with wire 7; no test-only host capability is exposed by the plugin.
wasmtime::component::bindgen!({
    inline: "package editor:plugin@0.1.0; interface host { request: func(payload: string) -> result<string, string>; } world plugin { import host; export dispatch: func(payload: string) -> result<string, string>; }",
    world: "plugin",
    require_store_data_send: true,
});

/// The adapter records accepted requests and can fail a bounded number of releases.
struct Host {
    /// WASI provides only standard component imports; the adapter grants no file preopens.
    wasi: WasiCtx,
    /// Per-instance WASI table, distinct from editor request identities.
    table: ResourceTable,
    /// Monotonic identities cannot alias a late terminal receipt.
    next: u64,
    /// Accepted requests remain owned until terminal delivery or successful CloseResource.
    owned: BTreeMap<u64, api::EditorOperation>,
    /// Immutable wire trace preserves request kind, target and original deadline for assertions.
    started: Vec<(api::ResourceHandle, api::EditorOperation, u32)>,
    /// Explicit fault budget; retries happen only when the test sends another user/lifecycle input.
    close_failures: usize,
}

impl WasiView for Host {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl editor::plugin::host::Host for Host {
    /// Reply with wire values only; no document edit or navigation is simulated as a native success.
    fn request(&mut self, payload: String) -> Result<String, String> {
        let request: api::Request = serde_json::from_str(&payload).unwrap();
        let result = match request.operation {
            api::Operation::Editor {
                operation,
                timeout_ms,
            } => {
                self.next += 1;
                let handle = api::ResourceHandle {
                    instance: "markdown-wire".into(),
                    scope: "workspace".into(),
                    resource: self.next,
                };
                self.owned.insert(handle.resource, operation.clone());
                self.started.push((handle.clone(), operation, timeout_ms));
                Ok(api::Value::Accepted(handle))
            }
            api::Operation::CancelRequest { handle, mode } => {
                if matches!(
                    self.owned.get(&handle.resource),
                    Some(api::EditorOperation::LocateViewport { .. })
                ) {
                    assert!(matches!(mode, api::CancelMode::StopWaiting));
                } else {
                    assert!(matches!(mode, api::CancelMode::TryTerminate));
                }
                if self.owned.contains_key(&handle.resource) {
                    Ok(api::Value::Cancellation(
                        api::CancellationEffect::NotExecuted,
                    ))
                } else {
                    Err(api::Failure::new(
                        api::ErrorCode::InvalidHandle,
                        "Already retired",
                    ))
                }
            }
            api::Operation::CloseResource { handle } => {
                if self.close_failures > 0 {
                    self.close_failures -= 1;
                    Err(api::Failure::new(
                        api::ErrorCode::OperationFailed,
                        "Injected close failure",
                    ))
                } else if self.owned.remove(&handle.resource).is_some() {
                    Ok(api::Value::Unit)
                } else {
                    Err(api::Failure::new(
                        api::ErrorCode::InvalidHandle,
                        "Already retired",
                    ))
                }
            }
            operation => panic!("Unexpected host operation: {operation:?}"),
        };
        Ok(serde_json::to_string(&api::Response {
            id: request.id,
            result,
        })
        .unwrap())
    }
}

/// One compiled package may supply many isolated component instances without reusing mutable guest state.
fn component() -> &'static (Engine, Component) {
    static COMPONENT: OnceLock<(Engine, Component)> = OnceLock::new();
    COMPONENT.get_or_init(|| {
        let path = std::env::var_os("NANOBUG_MARKDOWN_SDK_PACKAGE")
            .expect("Set NANOBUG_MARKDOWN_SDK_PACKAGE to the actual ZIP under test");
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut bytes = Vec::new();
        zip.by_name("markdown.wasm")
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let engine = Engine::new(Config::new().consume_fuel(true)).unwrap();
        let component = Component::new(&engine, bytes).unwrap();
        (engine, component)
    })
}

/// Current publications and editor requests are the only observable guest state used by these tests.
struct Guest {
    /// Each store contains independent guest state and its transport ledger.
    store: Store<Host>,
    /// Generated binding calls the real package's exported dispatch.
    plugin: Plugin,
    /// Public invocation correlation is separate from editor request identities.
    invocation: u64,
    /// Last published public scene, used for actual native action identities and visible feedback.
    view: ui::Document,
}

impl Guest {
    /// Start an isolated component with no preopens, environment inheritance or native side effects.
    fn new(text: &str) -> Self {
        let (engine, component) = component();
        let mut linker = Linker::new(engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).unwrap();
        Plugin::add_to_linker::<_, HasSelf<_>>(&mut linker, |state| state).unwrap();
        let mut store = Store::new(
            engine,
            Host {
                wasi: WasiCtx::builder().build(),
                table: ResourceTable::new(),
                next: 0,
                owned: BTreeMap::new(),
                started: Vec::new(),
                close_failures: 0,
            },
        );
        store.set_fuel(100_000_000).unwrap();
        let plugin = Plugin::instantiate(&mut store, component, &linker).unwrap();
        let mut guest = Self {
            store,
            plugin,
            invocation: 0,
            view: ui::Document::new(ui::Node::text("initial", "")),
        };
        guest
            .input(api::Input::Prepare {
                environment: Default::default(),
                api: api::Negotiated {
                    base: "1.0.0".parse().unwrap(),
                    capabilities: BTreeMap::new(),
                },
                snapshot: None,
            })
            .unwrap();
        guest.preview(Some(document("notes", "notes.md", 1)), text);
        guest
    }

    /// Preserve full public trees; compact patches are irrelevant without a FilePreview binding in this fixture.
    fn input(&mut self, message: api::Input) -> Result<(), api::Failure> {
        let output = self.call(message)?;
        if let Some(view) = output.views.into_iter().next() {
            self.view = view.document;
        }
        if let Some(mut patch) = output.view_patches.into_iter().next() {
            ui::incremental::restore(&self.view, &mut patch.document, &patch.reused).unwrap();
            self.view = patch.document;
        }
        Ok(())
    }

    /// Pure structure results use the same public lifecycle transport, independently of preview publications.
    fn call(&mut self, message: api::Input) -> Result<api::Output, api::Failure> {
        self.metered(message, 100_000_000).0
    }

    /// A diagnostic ceiling lets tests measure an over-budget call, then assert the unchanged production 100M limit.
    fn metered(
        &mut self,
        message: api::Input,
        ceiling: u64,
    ) -> (Result<api::Output, api::Failure>, u64) {
        self.invocation += 1;
        let payload = serde_json::to_string(&api::Invocation {
            id: self.invocation,
            message,
        })
        .unwrap();
        self.store.set_fuel(ceiling).unwrap();
        let reply = self
            .plugin
            .call_dispatch(&mut self.store, &payload)
            .unwrap()
            .unwrap();
        let completion: api::Completion = serde_json::from_str(&reply).unwrap();
        (completion.result, ceiling - self.store.get_fuel().unwrap())
    }

    /// Source revisions arrive through the same public notification as production previews.
    fn preview(&mut self, document: Option<api::DocumentVersion>, text: &str) {
        self.input(api::Input::Event {
            panel: Some("preview".into()),
            event: api::Notification::Preview {
                document,
                text: text.into(),
            },
        })
        .unwrap();
    }

    /// Native action identities come from the last published scene revision.
    fn click(&mut self, node: &str) {
        self.input(api::Input::Event {
            panel: Some("preview".into()),
            event: api::Notification::Ui(ui::UiEvent {
                revision: self.view.revision,
                node: node.into(),
                action: ui::Action::Click,
            }),
        })
        .unwrap();
    }

    /// Only the matching href in the current native node can authorize navigation.
    fn link(&mut self, node: &str, uri: &str) {
        self.input(api::Input::Event {
            panel: Some("preview".into()),
            event: api::Notification::Ui(ui::UiEvent {
                revision: self.view.revision,
                node: node.into(),
                action: ui::Action::Link { uri: uri.into() },
            }),
        })
        .unwrap();
    }

    /// Terminal delivery follows host retirement; late receipts may still be delivered after replacement.
    fn receipt(
        &mut self,
        handle: api::ResourceHandle,
        result: Result<api::EditorValue, api::Failure>,
    ) {
        self.store.data_mut().owned.remove(&handle.resource);
        self.input(api::Input::Event {
            panel: None,
            event: api::Notification::Request {
                handle,
                update: api::RequestUpdate::Completed { result },
            },
        })
        .unwrap();
    }

    /// Failure assertions inspect published localized text rather than private domain fields.
    fn feedback(&self) -> String {
        serde_json::to_string(&self.view).unwrap()
    }

    /// Repeat the existing lifecycle input to exercise failure retention before a logical State reset.
    fn prepare(&mut self) -> Result<(), api::Failure> {
        self.input(api::Input::Prepare {
            environment: Default::default(),
            api: api::Negotiated {
                base: "1.0.0".parse().unwrap(),
                capabilities: BTreeMap::new(),
            },
            snapshot: None,
        })
    }
}

/// Opaque document identity and UTF-8 text never become a second editable session.
fn document(id: &str, path: &str, revision: u64) -> api::DocumentVersion {
    api::DocumentVersion {
        id: id.into(),
        path: path.into(),
        revision,
    }
}

mod formatting;
mod navigation;
mod scrolling;
mod structure;
