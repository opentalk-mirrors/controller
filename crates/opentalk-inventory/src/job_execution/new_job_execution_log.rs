// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewJobExecutionLog {
    pub execution_id: SerialId,
    pub logged_at: DateTime<Utc>,
    pub log_level: LogLevel,
    pub log_message: String,
}
