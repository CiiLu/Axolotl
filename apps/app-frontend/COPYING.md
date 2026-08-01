# Copying

Axolotl Launcher's frontend is a modified version of Modrinth App's frontend. It is licensed under the GNU General Public License, Version 3 only, provided in [LICENSE](./LICENSE).

Copyright for the original work remains with Rinth, Inc. and the original contributors. Axolotl modifications are Copyright © 2026 Garbage Human Studio and were developed by Mystic Stars.

Axolotl Launcher is an independent, unofficial client. Modrinth is a trademark of Rinth, Inc. and is referenced only to identify API and file-format compatibility. Axolotl Launcher is not affiliated with or endorsed by Rinth, Inc.

## Schematic preview attribution

The local schematic preview uses [Deepslate](https://github.com/misode/deepslate) 0.26.0 for Minecraft blockstate and model mesh generation. Deepslate is distributed under the MIT License, and its package license is included with the installed dependencies.

The bundled blockstate data, block models, default block properties, and texture atlas are sourced from the public [Misode mcmeta](https://github.com/misode/mcmeta) dataset, which is generated from Minecraft: Java Edition client resources and Mojang's data generator. The current bundle is pinned to `summary@b8170fbc07725bf4930d189ad5dc16f70e09b9cd` and `atlas@a73f0316d9cea52a53381664328bda00e5fe79e4` (Minecraft `26.3-snapshot-6`) and can be reproduced with `scripts/axolotl/sync-schematic-resources.mjs`.

Minecraft and its original resources are Copyright Mojang AB. Their namespaced identifiers are retained only where required to identify compatible game content. Axolotl Launcher is not affiliated with or endorsed by Mojang or Microsoft.
