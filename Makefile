DESTDIR =
PREFIX = /usr/local

# Avoid double slash prefix
ifeq ($(DESTDIR),)
SAFE_PREFIX := $(PREFIX)
else
SAFE_PREFIX := $(DESTDIR)/$(PREFIX)
endif

SEARCH_PROVIDERS_DIR = $(SAFE_PREFIX)/share/gnome-shell/search-providers
LIBDIR = $(SAFE_PREFIX)/lib
DATADIR = $(SAFE_PREFIX)/share

SEARCH_PROVIDERS = $(wildcard providers/*.ini)

.PHONY: build
build:
	cargo build --release
	mkdir -p target/dbus-1 target/systemd
	sed "s:{LIBDIR}:$(LIBDIR):g" "dbus-1/de.swsnr.searchprovider.VSCode.service" > "target/dbus-1/de.swsnr.searchprovider.VSCode.service"
	sed "s:{LIBDIR}:$(LIBDIR):g" "systemd/de.swsnr.searchprovider.VSCode.service" > "target/systemd/de.swsnr.searchprovider.VSCode.service"

.PHONY: install
install:
	install -Dm644 -t $(SEARCH_PROVIDERS_DIR) $(SEARCH_PROVIDERS)
	install -Dm755 -t $(LIBDIR)/gnome-search-providers-vscode/ target/release/gnome-search-providers-vscode
	install -Dm644 -t $(LIBDIR)/systemd/user/ target/systemd/de.swsnr.searchprovider.VSCode.service
	install -Dm644 -t $(DATADIR)/dbus-1/services target/dbus-1/de.swsnr.searchprovider.VSCode.service

.PHONY: uninstall
uninstall:
	rm -f $(addprefix $(SEARCH_PROVIDERS_DIR)/,$(notdir $(SEARCH_PROVIDERS)))
	rm -rf $(LIBDIR)/gnome-search-providers-vscode/
	rm -f $(LIBDIR)/systemd/user/de.swsnr.searchprovider.VSCode.service
	rm -f $(DATADIR)/dbus-1/services/de.swsnr.searchprovider.VSCode.service
