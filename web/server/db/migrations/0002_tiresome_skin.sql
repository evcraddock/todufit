CREATE TABLE `group_calendar_tokens` (
	`id` text PRIMARY KEY NOT NULL,
	`group_doc_id` text NOT NULL,
	`token` text NOT NULL,
	`created_by_user_id` text NOT NULL,
	`created_at` integer NOT NULL,
	`revoked_at` integer,
	FOREIGN KEY (`created_by_user_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE UNIQUE INDEX `group_calendar_tokens_token_unique` ON `group_calendar_tokens` (`token`);