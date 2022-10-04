#!/bin/sh
rustfmt --check --edition 2021 rmf_site_format/src/lib.rs rmf_site_editor/src/lib.rs rmf_site_editor/src/main.rs
