PREFIX ?= $(HOME)/.local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
APPDIR = $(DATADIR)/applications
ICONDIR = $(DATADIR)/icons/hicolor/256x256/apps
MIMEDIR = $(DATADIR)/mime/packages

APP_NAME = fitslook

.PHONY: all build install uninstall clean

all: build

build:
	cargo build --release

install: build
	@bash install.sh "$(PREFIX)"

uninstall:
	@bash install.sh --uninstall "$(PREFIX)"

clean:
	cargo clean
