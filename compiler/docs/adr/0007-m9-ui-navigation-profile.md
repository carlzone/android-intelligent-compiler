# ADR 0007: M9 IR 0.2 and Activity navigation foundation

Status: implementation in progress; exit gate open.

AIC IR 0.2 is canonical. IR 0.1 remains accepted and can be upgraded by the
deterministic `aic-cli migrate --to 0.2` command. Migration changes only the
version header because the 0.1 grammar is a strict subset of 0.2.

An application may declare multiple Activities. The first is the launcher;
additional Activities are non-exported. Each Activity is independently verified
and emitted into a deterministic DEX unit. The APK uses `classes.dex`,
`classes2.dex`, and so on; this is supported without a multidex support library
because AIC's minSdk is 23. Navigation lowers to direct typed `Intent`,
`setClassName`, `startActivity`, and `finish` calls. Reflection remains forbidden.

The versioned capability catalog is embedded in compiler and host inputs. A
capability is not marked supported until parser, verifier, lowering, and tests
exist. M8 and M9 remain open pending the required compatibility/device evidence.
