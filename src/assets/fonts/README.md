# Bundled fonts

- `gothic-a1-{regular,medium,semibold,bold}.ttf`: Gothic A1, weights
  400/500/600/700. Complete official files from
  <https://github.com/google/fonts/tree/main/ofl/gothica1>, retrieved
  2026-11-26. License: `OFL-GothicA1.txt`.
- `oxanium-variable.ttf`: Oxanium, variable weights 200–800. Complete official
  file from <https://github.com/google/fonts/tree/main/ofl/oxanium>, retrieved
  2026-11-26. License: `OFL-Oxanium.txt`.
- `oxanium-wordmark.woff2`: existing RunDev logo subset; same Oxanium license.

The UI font stack uses Oxanium for supported Latin characters and numbers,
falling back to Gothic A1 for Korean. The logo keeps its existing Oxanium
subset; code and byte views keep their monospace stack. Fonts are served from
the app bundle with no CDN or runtime network request. Browsers load only the
weights used by each view.
