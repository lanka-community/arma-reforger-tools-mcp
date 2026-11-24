# game-files

Finds and opens Game files: the scripts, prefabs, configs, and other files inside the .pak archives of the installed game and of the Workshop mods that Workbench downloaded.

## Tools

- `game_find`: finds Game files by a part of their name or path, or by a class name, for example `CharacterCameraHandler` or `GameMode/SCR_`. Capital letters do not matter. It returns the real paths, for example `scripts/Game/Character/CharacterCameraHandlerComponent.c`, the file that defines `SCR_CharacterCameraHandlerComponent`.
  - Order: the full file name (with or without the extension), then names that start with the query, then names that have it inside, then folder paths that have it inside. Game files come before mod files, and shorter paths before longer ones.
  - A class name that starts with `SCR_` also finds the file without the prefix: `SCR_CharacterCameraHandlerComponent` finds `CharacterCameraHandlerComponent.c` first.
  - Each path says where it comes from: `(game)` or `(mod tvtLobby)`.
  - `limit` sets how many paths the list shows, from 1 to 100 (default 20). A longer list is cut, and the first line says how many match.
  - When no file matches, the answer says how many files it searched, in the game and in which mods.
  - An empty query gives an error with an example.
- `game_read`: opens one Game file by its `path` and returns its full text. The first line gives the real path, the source, and the size in bytes, for example `scripts/Game/Character/CharacterCameraHandlerComponent.c (game, 43980 bytes)`.
  - Capital letters do not matter: `Scripts/Game/Character/CharacterCameraHandlerComponent.c` opens the file and gives the real path, `scripts/...`.
  - A path that no file has gives the 5 closest real paths. The file name counts most, and a `SCR_` prefix that the file name does not have is ignored. Example: `scripts/Game/Characters/SCR_CharacterControllerComponentt.c` gives `scripts/Game/Character/SCR_CharacterControllerComponent.c` first.
  - A path copied from `game_find` with its source, for example `Configs/Shared.conf (mod tvtLobby)`, works. So do back slashes, and a Prefab path with its `{GUID}` in front.
  - When the same path is in the game and in mods, `source` says which one to open: `game`, a mod name, for example `tvtLobby`, or a mod ID. Without it, the game opens first, and the answer names the other sources.
  - The game has 34 paths in two .pak files, for example `Prefabs/AI/SCR_AIWorld.et` in `data/data007.pak` and `rally/data.pak`. The first .pak file in folder order opens, and the answer names both .pak files.
  - A file with a zero byte in its first 8,000 bytes is binary, for example a texture. It gives an error, not its bytes.
  - The server cuts an answer at 100,000 characters, and the end says so: "(Cut at 100,000 of N characters.)". A file with a full size above 64 MB gives an error before it is unpacked.
- `game_list`: lists the game and each downloaded mod with its file count and the full size of its files in bytes, for example `- tvtLobby (ID 5F2A6A1D2A3B4C5D): 2 files, 1009 bytes`. The full size is the size after unpacking, from the FILE table, so the Tool unpacks no file.
  - `source` lists the files of one source in path order, each with its full size, for example `- Scripts/Big.c (1000 bytes)`. The first line gives the file count and the size of all of them. `source` is `game`, a mod name such as `tvtLobby` or `mod tvtLobby`, or a mod ID. Capital letters do not matter. `game_read` takes the same `source` values (`Index::is_source`).
  - `path` counts and lists only the files with that part in their path, for example `scripts/` for the scripts of each mod. Back slashes work.
  - `limit` sets how many files of a source the list shows, from 1 to 100 (default 20). A longer list is cut, and the first line says so.
  - An unknown source gives an error that lists the game and each mod with its ID.

## Settings

Set them in the `env` of Claude Code's `settings.json`, or let the setup write them (see "Setup"). A path works in Windows form (`D:\Games\...`) and in WSL form (`/mnt/d/Games/...`). Restart Claude Code after a change.

- `REFORGER_GAME_PATH`: the game folder, the folder that holds `ArmaReforger.exe` and `addons`. Without it, or when it has no `addons` folder with .pak files, `game_find`, `game_read`, and `game_list` give an error that names the setting. When Steam names a game folder, the error names it too, with the command of the setup: `Found the game at /mnt/d/Games/steamapps/common/Arma Reforger. Set REFORGER_GAME_PATH to it, or ask the user to run the setup in a terminal: "<plugin folder>/target/release/reforger" setup.` The Tools never use the found folder by themselves (ADR 0007).
- `REFORGER_WORKBENCH_DOCUMENTS_PATH`: the Workbench folder in Documents, for example `C:\Users\<you>\Documents\My Games\ArmaReforgerWorkbench`. Workbench downloads Workshop mods to its `addons` folder. Without it, `game-files` finds the folder the same way as `workbench` (`src/workbench_documents.rs`).
- `REFORGER_WORKBENCH_PROFILE_PATH`: a second folder that holds `addons` with Workshop mods, for example `D:\DownloadedMods`, the folder where the game downloads mods. Without it and without a Documents folder, `game_find`, `game_read`, and `game_list` use the game only, and `game_find` says so.

## Setup

`reforger setup` finds the two folders through Steam and writes them to the settings. Run it in a terminal, in the plugin folder: `target/release/reforger setup`. In WSL, run it in WSL, so that it writes the paths in WSL form (ADR 0006).

1. It finds the Steam folder: first the folder that the Windows registry names (`reg.exe query HKCU\Software\Valve\Steam /v SteamPath`, which works from WSL too), then the default folder `C:\Program Files (x86)\Steam`. The first one with `steamapps/libraryfolders.vdf` counts. On the test machine, Steam is in `D:\Games`.
2. `REFORGER_GAME_PATH`: `libraryfolders.vdf` names the library of Arma Reforger (app 1874880), and `steamapps/appmanifest_1874880.acf` in that library names its folder under `steamapps/common`.
3. `REFORGER_WORKBENCH_PROFILE_PATH`: `userdata/<user>/config/localconfig.vdf` holds the launch options of the game. The folder of `-addonDownloadDir "D:\DownloadedMods"` holds `addons` with the mods that the game downloads. When Steam has more than one user, the file that changed last counts first. Without the option, the setup uses the Workbench folder in Documents (`C:\Users\<you>\Documents\My Games\ArmaReforgerWorkbench`, or `REFORGER_WORKBENCH_DOCUMENTS_PATH`).
4. It shows each file it read and what it found, and the changes to the `env` of `~/.claude/settings.json` (or `settings.json` in `CLAUDE_CONFIG_DIR`). It writes only after the answer `y`. Every other key and the order of the keys stay as they were. When `settings.json` is a link, it writes the file behind the link. When the file is not valid JSON, it writes nothing and says so.
5. When it finds nothing, it lists the files it read and says how to set the settings by hand.

`src/steam/` holds the search, and `src/setup/` the command. A damaged Steam file gives a line with the file name and the line of the damage, never a crash. Restart Claude Code after the setup.

## Data

The file lists come from the FILE table of each .pak archive:

- The game: every `.pak` file in `<game>/addons` and one folder down, for example `addons/data/data007.pak`. The game has 19 archives with about 251,000 files.
- The mods: every `.pak` file in `<Documents folder>/addons/<Name>_<ID>/` and in `<profile>/addons/<Name>_<ID>/`. The mod name is the folder name without the 16-digit ID. When the same mod ID is in both folders, the mod is listed once, from the Documents folder.

The first `game_find`, `game_read`, or `game_list` call reads all file lists, outside the server thread, in about a second. Before each later call, `game-files` lists the mod folders in both `addons` folders and the .pak files in each, with their size and modified time. It reads no .pak file for this. When one of them differs from the last build, it reads all file lists again, so a mod that Workbench downloads shows with no restart, also when its folder came before its .pak file. A damaged .pak file is skipped, and the answer names it.

`pak.rs` reads the FILE table and the content of one file. It is translated from `src/pak/reader.ts` and `src/pak/vfs.ts` of enfusion-mcp, with three changes:

- The `offset` of a file is a position from the start of the .pak file, not from the DATA chunk.
- A compressed file is zlib with a header, so it is unpacked with zlib `inflate`, not raw deflate. The old raw deflate failed with "invalid block type" on most scripts.
- Every size, offset, child count, folder depth, and path length is checked against the file length or a limit before the reader uses it. A damaged file gives an error, never a crash.
