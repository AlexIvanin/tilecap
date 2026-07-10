NAME     := tilecap
VERSION  := $(shell grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
PREFIX   ?= /usr/local
BINDIR   ?= $(PREFIX)/bin

.PHONY: all build test clean release \
        deb rpm \
        install uninstall \
        dist dist-deb dist-rpm dist-arch \
        git-archive

all: build

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

release: build
	mkdir -p dist
	cp target/release/$(NAME) dist/
	strip dist/$(NAME)

deb:
	cargo deb -p $(NAME)
	mkdir -p dist
	cp target/debian/*.deb dist/

rpm:
	cargo generate-rpm
	mkdir -p dist
	cp target/rpm/*.rpm dist/

arch:
	mkdir -p dist
	cp PKGBUILD dist/

dist: release
	cd target/release && \
		tar czf ../../dist/$(NAME)-$(VERSION)-x86_64-linux.tar.gz $(NAME)

dist-deb: deb
dist-rpm: rpm
dist-arch: arch

install:
	@if [ ! -f target/release/$(NAME) ]; then \
		echo "=== Run 'make build' first (as user), then 'doas make install' ==="; \
		exit 1; \
	fi
	install -Dm755 target/release/$(NAME) $(DESTDIR)$(BINDIR)/$(NAME)

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/$(NAME)

git-archive:
	git archive --format=tar.gz -o dist/$(NAME)-$(VERSION).tar.gz \
		--prefix=$(NAME)-$(VERSION)/ HEAD

dist-all: dist dist-deb dist-rpm dist-arch git-archive

release-all: release dist-all

.PHONY: help
help:
	@echo "tilecap $(VERSION)"
	@echo ""
	@echo "Targets:"
	@echo "  build          cargo build --release"
	@echo "  test           cargo test"
	@echo "  clean          cargo clean"
	@echo "  install        install binary to PREFIX/bin"
	@echo "  uninstall      remove binary"
	@echo ""
	@echo "  release        strip + copy to dist/"
	@echo "  dist           tar.gz archive of binary"
	@echo "  deb            .deb package"
	@echo "  rpm            .rpm package"
	@echo "  arch           PKGBUILD for Arch"
	@echo "  git-archive    git archive of source"
	@echo "  dist-all       all of the above"
	@echo "  release-all    release + dist-all"
	@echo ""
	@echo "Variables:"
	@echo "  PREFIX=$(PREFIX)"
	@echo "  BINDIR=$(BINDIR)"
