install:
	sudo cp target/release/ptu /usr/local/bin/

build-install:
	cargo b -r
	sudo cp target/release/ptu /usr/local/bin/
