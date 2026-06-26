# DEPRECATED: ACL Management

> Commands and settings mentioned in this section are no longer applicable and will be removed with the upcoming release.

{{ product_name }} uses an in-memory Access Control List to efficiently track permissions. The controller maintains that list in
its database. Controllers will synchronize changes to the ACL by sending changesets to each other using RabbitMQ.

## Configuration

The section in the [configuration file](../core/configuration.md) is called `authz`.

| Field                     | Type   | Required | Default value | Description                                                                                                                               |
| ------------------------- | ------ | -------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `synchronize_controllers` | `bool` | no       | `true`        | Must `true` when using multiple controllers. This should be set to `false` to avoid unnecessary work, when only using a single controller |

## `opentalk-controller acl` subcommand

This subcommand is no longer necessary.

## `opentalk-controller fix-acl` subcommand

This subcommand is no longer necessary.
