starboard = starboard
    .description = Manage starboard settings

starboard-enable = enable
    .description = Enable the starboard
    .msg = Starboard enabled

starboard-disable = disable
    .description = Disable the starboard
    .msg = Starboard disabled

starboard-threshold = threshold
    .description = Set the number of reactions required to post to starboard
    .value = value
    .value-description = Minimum reaction count
    .msg-invalid = Threshold must be >= 1
    .msg-set = Threshold set: { $value }

starboard-emoji = emoji
    .description = Set the emoji used to trigger the starboard
    .value = value
    .value-description = Standard Unicode emoji or custom server emoji
    .msg-invalid = Emoji could not be recognized. Use a standard Unicode emoji or a custom server emoji.
    .msg-set = Emoji set: { $value }

starboard-channel = channel
    .description = Set the channel where starred messages are posted
    .value = value
    .value-description = Target channel
    .msg-set = Channel set: { $value }