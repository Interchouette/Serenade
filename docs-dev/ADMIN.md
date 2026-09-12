# Optional Admin CRUD bundle (parked)

Scope-only design note for an EasyAdmin-shaped **Admin CRUD** Serenade bundle. **No crate ships from this note.** Implement only after an explicit order ([#124](https://github.com/Interchouette-ITC/Serenade/issues/124)).

Core stance (admin generator is **not** in the kernel / `FrameworkBundle`): [KERNEL.md](KERNEL.md#admin-back-office), [#122](https://github.com/Interchouette-ITC/Serenade/issues/122), [#123](https://github.com/Interchouette-ITC/Serenade/issues/123).

## Why parked

Modern Symfony keeps CRUD admin in the **ecosystem** (EasyAdmin, Sonata, Maker stubs), not in FrameworkBundle. Serenade follows the same split: Form, CSRF, security, and session login are framework primitives; a generated admin UI is optional composition.

Until a crate exists, apps keep hand-authored BO pages (MyFeed `/admin`, product Angular / Leptos admin hosts).

## What the bundle would own

| Piece | Bundle | App / product |
| --- | --- | --- |
| Config → list / show / new / edit / delete routes | Yes | Declares resources |
| Field descriptors (label, type, sortable, filterable) | Yes | Resource field map |
| Filters on list (equality, range, search) | Yes | Which filters are enabled |
| Forms for create / edit | Reuses `serenade-form` + CSRF | Entity bind / persist |
| Access checks | Reuses security voters / firewall | Roles / subjects |
| HTML / SPA chrome | Thin HTML helpers or none | Host UI (templates or SPA) |
| Persistence | Calls app repositories | `ProductRepository`, etc. |

Suggested package shape when ordered: separate Cargo crate (e.g. `serenade-admin`), own `BundleInterface` + `Extension`, recipe under `config/packages/admin.toml`. **Not** registered by `FrameworkBundle` by default.

## Config-driven shape (concepts only)

Illustrative TOML (not implemented):

```toml
# config/packages/admin.toml (future)
[admin.resources.product]
path_prefix = "/admin/products"
entity = "product"
list_fields = ["id", "name", "price_cents", "active"]
filter_fields = ["name", "active"]
form_fields = ["name", "price_cents", "active"]
```

Behavior concepts:

1. **List** - table of configured fields; optional filters; pagination contract left to the app or a later shared helper.
2. **Show** - read-only field set for one id.
3. **New / Edit** - `serenade-form` form built from `form_fields`; CSRF default-on; POST bind → app save.
4. **Delete** - POST/DELETE with CSRF; app delete; redirect to list.

Resource handlers stay app-owned: load by id, save, delete, list query. The bundle would only orchestrate HTTP + forms + config.

## Must reuse

- [FORMS.md](FORMS.md) - bind, validate, render, CSRF
- [SECURITY.md](SECURITY.md) - firewall and/or `login` / `SessionTokenMiddleware` for HTML admin
- [BUNDLES.md](BUNDLES.md) - third-party / first-party bundle policy

## Non-goals (until ordered)

- Shipping `serenade-admin` (or any admin crate) in this repository
- Auto-wiring CRUD admin into `FrameworkBundle`
- Porting Symfony 1 `sfPropelAdminGenerator` / Doctrine admin generator
- Replacing product SPA admin hosts with generated HTML
- Claiming Redis / multi-tenant admin as a v1 product

## When to order implementation

Greg (or product) opens an implementation issue that:

1. Names the crate and recipe
2. Picks HTML-first vs API-only admin JSON for SPA hosts
3. Lists the first dogfood app (MyFeed moderation is **not** required to migrate)

Until then this file is the locked scope only.
