# Regression inputs

`platform-specification.md` preserves the exact Nanobug platform specification used by the complete native preview regression at extraction. The fixture is test data, excluded from the plugin ZIP. It keeps the previously tested document size and syntax without a dependency on a sibling host checkout. Update it only when deliberately changing this regression's input; record the digest and rerun the parser/native document checks.
