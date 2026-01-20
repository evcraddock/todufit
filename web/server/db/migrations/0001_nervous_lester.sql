CREATE TABLE `group_invites` (
	`id` text PRIMARY KEY NOT NULL,
	`token` text NOT NULL,
	`group_doc_id` text NOT NULL,
	`group_name` text NOT NULL,
	`inviter_user_id` text NOT NULL,
	`invitee_email` text NOT NULL,
	`expires_at` integer NOT NULL,
	`created_at` integer NOT NULL,
	`accepted_at` integer,
	`accepted_by_user_id` text,
	FOREIGN KEY (`inviter_user_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`accepted_by_user_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `group_invites_token_unique` ON `group_invites` (`token`);