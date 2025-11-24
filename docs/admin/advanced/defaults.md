# Default and Fallback Values

## Features

In the [configuration file](../core/configuration.md), the format of a [`feature`](./modules.md#features) is: `[<module>::]<feature>`.
A missing module specifier defaults to `"core"`. The features currently supported are:

- `core::call_in`
- `integration::outlook`

The [`modules`](./modules.md#opentalk-controller-modules-subcommand) subcommand outputs all modules
available in the {{ product_name }} controller, including the features that can be enabled or disabled.

## Configuration

The section in the [configuration file](../core/configuration.md) is called `defaults`.

| Field                              | Type       | Required | Default value | Description                                                                  |
| ---------------------------------- | ---------- | -------- | ------------- | ---------------------------------------------------------------------------- |
| `user_language`                    | `string`   | no       | `"en-US"`     | Default language of a new user                                               |
| `screen_share_requires_permission` | `bool`     | no       | `false`       | When `true`, screen sharing requires explicit permission                     |
| `timezone`                         | `string`   | no       | see below     | The global timezone of the controller, in IANA format (e.g. "Europe/Berlin") |
| `disabled_features`                | `string[]` | no       | `[]`          | A list of disabled features in the controller                                |

The `timezone` field sets the timezone used by the system and as the users' default. If not set here, the `TZ` environment variable and the operating system
are consulted in this order, finally falling back to "Etc/UTC").

### Examples

#### Set the global timezone and disable the `core::call_in` and `integration::outlook` features

```toml
[defaults]
timezone = "Europe/Berlin"
disabled_features = ["core::call_in", "integration::outlook"]
```

#### Set default user language, require explicit screen share permissions and disable the `core::call_in` feature

```toml
[defaults]
user_language = "de-DE"
screen_share_requires_permission = true
disabled_features = ["call_in"]
```
