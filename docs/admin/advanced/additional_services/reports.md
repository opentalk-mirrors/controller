---
sidebar_position: 304
---

:::warning

The report configuration is deprecated and will be removed or changed in the future

:::

# Report Generator

The [OpenTalk report generator](https://gitlab.opencode.de/opentalk/terdoc) is a service whose main purpose is to generate reports (e.g. in PDF).

When no reports generator is configured, it is disabled.

## Deploy report generator

The report generator has to be configured and started separately.

You can clone the repository and build the Docker image yourself or download the image directly from the [container registry](https://gitlab.opencode.de/opentalk/terdoc/container_registry).

## Configuration

The section in the [configuration file](../../core/configuration.md) is called `reports`.

| Field     | Type     | Required | Default value | Description                                                |
| --------- | -------- | -------- | ------------- | ---------------------------------------------------------- |
| `url`     | `string` | yes      | -             | The URI address where the reports generator can be reached |

### Examples

#### Default setup

```toml
[reports]
url = "http://localhost:6560"
```
