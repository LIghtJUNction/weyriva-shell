.PHONY: check test install update uninstall

check:
	./scripts/check.sh

test:
	python3 -m unittest discover -s tests -v

install:
	./scripts/install.sh

update:
	./scripts/update.sh

uninstall:
	./scripts/uninstall.sh
