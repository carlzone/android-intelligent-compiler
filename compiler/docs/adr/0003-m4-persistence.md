# ADR 0003: M4 persistence and capability boundary

Status: accepted

M4 uses Android app-private `SharedPreferences` as its typed key/value backend and the platform SQLite implementation as its record store. Both capabilities require explicit verified IR declarations and map to no Android manifest permissions. Declared-but-unused and used-but-undeclared capabilities are compiler errors.

Preferences support non-null `i32`, `bool`, and `string` values with literal defaults. SQLite is limited to schema version 1, non-null scalar columns, and one `i32 primary_key auto_increment` column per table. Generated queries are compiler-owned and bind application values; raw SQL is not part of the IR. SQLite statements are closed after use.

M4 does not implement migrations. During development, an incompatible schema change requires clearing app data or uninstalling the generated application. Network, files, notifications, camera, navigation, and permission-bearing capabilities remain deferred.
