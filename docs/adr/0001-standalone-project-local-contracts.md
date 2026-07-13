# Keep Workmen standalone and store contracts with each game

Workmen is a standalone game-asset workbench that can open any game project rather than being embedded in one engine or repository. Reusable Presets belong to Workmen, while resolved Profiles are exported into each game repository so the asset contract is versioned and reviewed with the game that depends on it. This preserves cross-project reuse without allowing a later Preset change to silently alter an existing game's shipping rules.
