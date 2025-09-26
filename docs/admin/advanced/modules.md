# Modules

Modules provide functionality that can be used inside meetings.

<!-- begin:fromfile:modules/module-features-documentation.md -->

## Module `core`

Handles core meeting functionality

### Features

The following features can be configured for the module. All features are enabled by default and can be disabled either [by configuration](https://docs.opentalk.eu/admin/controller/advanced/defaults/) or [by tariff](https://docs.opentalk.eu/admin/controller/advanced/tariffs/).

#### `core::call_in`

Enables telephone call-in functionality for meetings.

#### `core::guests_allowed`

Allow guest access. With this feature enabled, guests are allowed to join meetings through invite links. When turned off, no invite links are generated, and invite links that existed before will be invalid.

#### `core::storage_upgradable`

Communicates to the frontend that the user's storage can be upgraded. Frontend will then show the corresponding link to the account management if the user's storage is close to the limit. This feature is usually configured differently the tariffs. If a user has a tariff which already provides the maximum available storage space, then that feature should be disabled. For all other tariffs it should be on.

## Module `breakout`

Handles breakout room functionality

### Features

This module does not provide any configurable features.

## Module `moderation`

Handles moderation functionality

### Features

This module does not provide any configurable features.

## Module `echo`

Used for internal connection checking and development

### Features

This module does not provide any configurable features.

## Module `recording`

Handles recording functionality. The `recording_service` must be enabled as well for recording to work properly, it performs communication with the [recordings service](https://docs.opentalk.eu/admin/recorder/).

### Features

The following features can be configured for the module. All features are enabled by default and can be disabled either [by configuration](https://docs.opentalk.eu/admin/controller/advanced/defaults/) or [by tariff](https://docs.opentalk.eu/admin/controller/advanced/tariffs/).

#### `recording::record`

Allows creation of recordings for meetings

#### `recording::stream`

Allows streaming meetings to streaming services

## Module `recording_service`

Handles communication between the meeting room and the recorder. This is required if the `recording` module is enabled.

### Features

This module does not provide any configurable features.

## Module `chat`

Handles room chat functionality

### Features

This module does not provide any configurable features.

## Module `legal_vote`

Handles the legal-vote functionality

### Features

This module does not provide any configurable features.

## Module `automod`

Handles auto-moderation functionality such as the talking stick

### Features

This module does not provide any configurable features.

## Module `livekit`

Handles Livekit media streams coordination and integration

### Features

This module does not provide any configurable features.

## Module `polls`

Handles meeting polls functionality

### Features

This module does not provide any configurable features.

## Module `meeting_notes`

Handles meeting note editing and viewing functionality

### Features

This module does not provide any configurable features.

## Module `shared_folder`

Handles shared folder integration. This allows automatic creation of shares on a NextCloud instance using the [OCS API](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/OCS/ocs-api-overview.html).

### Features

This module does not provide any configurable features.

## Module `timer`

Handles timer functionality including the coffee-break timer.

### Features

This module does not provide any configurable features.

## Module `whiteboard`

Handles whiteboard integration. The whiteboard is a collaborative drawing board that can be used during the meeting.

### Features

This module does not provide any configurable features.

## Module `meeting_report`

Handles generation of meeting reports, e.g. participant list export

### Features

This module does not provide any configurable features.

## Module `subroom_audio`

Handles sub-room audio, allowing participants to talk to each other in a separate audio group.

### Features

This module does not provide any configurable features.

## Module `training_participation_report`

Handles training participation report functionality. Participants are asked to confirm their presence repeatedly at pre-configured time intervals. These confirmations are documented in the training participation report which is created automatically at the end of the meeting.

### Features

This module does not provide any configurable features.

## Module `asset_storage`

Provides infomation about the asset storage.

### Features

This module does not provide any configurable features.

<!-- end:fromfile:modules/module-features-documentation.md -->

## `opentalk-controller modules` subcommand

This command outputs all modules available in the {{ product_name }} controller, including
the features that can be enabled or disabled:

```text
opentalk-controller modules list
```

Example output:

<!-- begin:fromfile:cli-usage/opentalk-controller-modules-list.md -->

```text
core: ["call_in", "guests_allowed", "storage_upgradable"]
breakout: []
moderation: []
echo: []
recording: ["record", "stream"]
recording_service: []
chat: []
legal_vote: []
automod: []
livekit: []
polls: []
meeting_notes: []
shared_folder: []
timer: []
whiteboard: []
meeting_report: []
subroom_audio: []
training_participation_report: []
asset_storage: []
```

<!-- end:fromfile:cli-usage/opentalk-controller-modules-list.md -->
