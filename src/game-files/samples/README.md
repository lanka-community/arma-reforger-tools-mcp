# Bohemia's sample mods

The text files of the sample mods of Bohemia Interactive. The server program holds them. `game_find` and `game_read` show each sample mod as a source, for example `(sample SampleMod_NewWeapon)`, and `game_read` opens a file of it with `source=SampleMod_NewWeapon`.

- Source: https://github.com/BohemiaInteractive/Arma-Reforger-Samples
- Commit: `83f12390d3dd61ed834d379f621520ed9f1b891d`
- Copyright: © 2022 Bohemia Interactive a.s. All rights reserved.
- License: Arma Public License (APL), in `LICENSE`. The full text is at https://www.bohemia.net/community/licenses/arma-public-license. `SampleMod_Main/license.txt` is the license file that the sample `SampleMod_Main` comes with.

The MIT license of this repo does not cover this folder. Use these files only for non-commercial work on Arma games, as the APL says. Bohemia Interactive does not support or endorse this plugin.

`cargo xtask import --samples <clone>` copied only the text files (`.c`, `.et`, `.ct`, `.conf`, `.layout`, `.gproj`, `.meta`, `.ent`, `.layer`, `.st`) and the `README.md` and `license.txt` files. It did not change them. The models, textures, sounds, animations, and other binary files are not here. Get them from the source repo.
