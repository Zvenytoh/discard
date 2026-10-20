# Bundled Vencord

Unmodified official browser assets, fetched 2026-10-20 from the devbuild release.
Source commit: 32c90633ce6a95d7e56c95479efbb431bacf1eac.
Corresponding source: https://github.com/Vendicated/Vencord/tree/32c90633ce6a95d7e56c95479efbb431bacf1eac
Source archive: https://github.com/Vendicated/Vencord/archive/32c90633ce6a95d7e56c95479efbb431bacf1eac.tar.gz

assets.json records GitHub asset IDs and SHA-256 digests. The bundle header also
identifies 32c9063. Browser.js and browser.css are embedded into the executable;
they are not fetched at runtime. Discard supplies getRendererCss from bundled CSS
instead of using the extension metadata loader. Vencord settings remain in the
existing browser profile. License: GPL-3.0-or-later, see LICENSE and
browser.js.LEGAL.txt for bundled third-party notices. Discard source is distributed
under GPL-3.0-or-later as well. Plugins can independently access the network.

To update: obtain a new upstream build, verify every GitHub SHA-256 digest, resolve
the bundle header to its full source commit, update assets.json and COMMIT, keep
licenses/notices, and validate the integration before publishing a new Discard.
