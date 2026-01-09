## Mi Code Added Memories
- In this project, treat a crate as a RIDL module only if the dependency path's src/ directory contains at least one *.ridl file; otherwise exclude it from registry-driven RIDL aggregation.
- In this project (mquickjs), C API registration cannot happen at runtime; registration must be done at compile time. This constraint is the root reason for the symbol keep-alive + build-time aggregation design; avoid suggesting runtime QuickJS C API registration.
