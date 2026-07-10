# Copyright 2026 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

CRATES="
	adler2@2.0.1
	as-raw-xcb-connection@1.0.1
	bitflags@1.3.2
	bitflags@2.13.0
	cfg-if@1.0.4
	crc32fast@1.5.0
	equivalent@1.0.2
	fdeflate@0.3.7
	flate2@1.1.9
	gethostname@1.1.0
	hashbrown@0.17.1
	indexmap@2.14.0
	libc@0.2.186
	linux-raw-sys@0.12.1
	miniz_oxide@0.8.9
	proc-macro2@1.0.106
	png@0.17.16
	quote@1.0.46
	rustix@1.1.4
	serde@1.0.228
	serde_core@1.0.228
	serde_derive@1.0.228
	serde_spanned@0.6.9
	simd-adler32@0.3.9
	syn@2.0.118
	toml@0.8.23
	toml_datetime@0.6.11
	toml_edit@0.22.27
	toml_write@0.1.2
	unicode-ident@1.0.24
	winnow@0.7.15
	x11rb@0.13.2
	x11rb-protocol@0.13.2
	xcursor@0.3.10
"

inherit cargo

DESCRIPTION="Minimal screenshot tool for tiling window managers"
HOMEPAGE="https://github.com/kira/tilecap"
SRC_URI="
	https://github.com/kira/tilecap/archive/v${PV}.tar.gz -> ${P}.tar.gz
	${CARGO_CRATE_URIS}
"

LICENSE="MIT"
SLOT="0"
KEYWORDS="~amd64"
IUSE="+clipboard"

DEPEND="
	x11-libs/libX11
	x11-libs/libxcb
"
RDEPEND="
	${DEPEND}
	clipboard? ( x11-misc/xclip )
"
BDEPEND="
	>=virtual/rust-1.77
"

QA_FLAGS_IGNORED="usr/bin/${PN}"

src_install() {
	cargo_src_install
	dodoc README.md
}
