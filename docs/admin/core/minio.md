# File Storage (MinIO)

The {{ product_name }} Controller uses [MinIO](https://min.io/) as its object storage.

## Configuration

The section in the [configuration file](./configuration.md) is called `minio`.

| Field              | Type     | Required | Default value | Description                                           |
| ------------------ | -------- | -------- | ------------- | ----------------------------------------------------- |
| `uri`              | `string` | yes      | -             | The URI address where the MinIO server can be reached |
| `bucket`           | `string` | yes      | -             | The default bucket name for object storage            |
| `access_key`       | `string` | yes      | -             | The unique username for the MinIO user                |
| `secret_key`       | `string` | yes      | -             | The password corresponding to the access key          |
| `force_path_style` | `bool`   | no       | false         | Force path style                                      |
| `region`           | `string` | no       | unknown       | AWS region                                            |

### Examples

#### Default setup

```toml
[minio]
uri = "http://localhost:9555"
bucket = "controller"
access_key = "minioadmin"
secret_key = "minioadmin"
force_path_style = false
region = "unknown"
```
