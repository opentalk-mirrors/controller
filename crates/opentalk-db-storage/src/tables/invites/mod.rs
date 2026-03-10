// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

//! Contains invites table structs

mod invite;
mod invite_code_serial_id;
mod new_invite;
mod update_invite;

pub use invite::Invite;
pub use invite_code_serial_id::InviteCodeSerialId;
pub use new_invite::NewInvite;
pub use update_invite::UpdateInvite;
