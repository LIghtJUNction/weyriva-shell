.PHONY: check test install update uninstall

check:
	cargo fmt --all -- --check
	cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
	cargo test --locked --workspace --all-targets --all-features

test:
	cargo test --locked --workspace --all-targets --all-features

install:
	./install.sh

update:
	./scripts/update.sh

uninstall:
	./scripts/uninstall.sh
