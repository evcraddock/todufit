import nodemailer from 'nodemailer'

// Check if we're in dev mode (log emails instead of sending)
const isDevMode = process.env.SMTP_DEV_MODE === 'true'

// Create transporter (only if not in dev mode)
let transporter: nodemailer.Transporter | null = null

if (!isDevMode) {
  transporter = nodemailer.createTransport({
    host: process.env.SMTP_HOST,
    port: Number(process.env.SMTP_PORT) || 587,
    secure: process.env.SMTP_PORT === '465',
    auth: {
      user: process.env.SMTP_USER,
      pass: process.env.SMTP_PASS,
    },
  })
}

interface MagicLinkEmailOptions {
  to: string
  token: string
  expiresInMinutes: number
}

interface GroupInviteEmailOptions {
  to: string
  token: string
  groupName: string
  inviterEmail: string
  expiresInHours: number
}

/**
 * Send a magic link email for authentication
 */
export async function sendMagicLinkEmail(options: MagicLinkEmailOptions): Promise<void> {
  const { to, token, expiresInMinutes } = options
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  const magicLink = `${publicUrl}/auth/verify?token=${encodeURIComponent(token)}`

  const fromName = process.env.SMTP_FROM_NAME || 'todu-fit'
  const fromEmail = process.env.SMTP_FROM_EMAIL || 'noreply@example.com'

  const subject = 'Your login link for todu-fit'
  const text = `
Click the link below to log in to todu-fit:

${magicLink}

This link will expire in ${expiresInMinutes} minutes.

If you didn't request this link, you can safely ignore this email.
`.trim()

  const html = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
    .container { max-width: 500px; margin: 0 auto; padding: 20px; }
    .button { display: inline-block; padding: 12px 24px; background: #3b82f6; color: white; text-decoration: none; border-radius: 6px; }
    .footer { margin-top: 20px; font-size: 14px; color: #666; }
  </style>
</head>
<body>
  <div class="container">
    <h2>Log in to todu-fit</h2>
    <p>Click the button below to log in:</p>
    <p><a href="${magicLink}" class="button">Log In</a></p>
    <p class="footer">
      This link will expire in ${expiresInMinutes} minutes.<br>
      If you didn't request this link, you can safely ignore this email.
    </p>
  </div>
</body>
</html>
`.trim()

  if (isDevMode) {
    // In dev mode, log the email instead of sending
    console.log('\n=== MAGIC LINK EMAIL (DEV MODE) ===')
    console.log(`To: ${to}`)
    console.log(`Subject: ${subject}`)
    console.log(`Magic Link: ${magicLink}`)
    console.log('===================================\n')
    return
  }

  if (!transporter) {
    throw new Error('Email transporter not configured')
  }

  await transporter.sendMail({
    from: `"${fromName}" <${fromEmail}>`,
    to,
    subject,
    text,
    html,
  })
}

/**
 * Send a group invitation email
 */
export async function sendGroupInviteEmail(options: GroupInviteEmailOptions): Promise<void> {
  const { to, token, groupName, inviterEmail, expiresInHours } = options
  const publicUrl = process.env.PUBLIC_URL || 'http://localhost:5173'
  const inviteLink = `${publicUrl}/auth/invite/accept?token=${encodeURIComponent(token)}`

  const fromName = process.env.SMTP_FROM_NAME || 'todu-fit'
  const fromEmail = process.env.SMTP_FROM_EMAIL || 'noreply@example.com'

  const subject = `${inviterEmail} invited you to join "${groupName}" on todu-fit`
  const text = `
${inviterEmail} has invited you to join "${groupName}" on todu-fit.

Click the link below to accept the invitation:

${inviteLink}

This invitation will expire in ${expiresInHours} hours.

If you didn't expect this invitation, you can safely ignore this email.
`.trim()

  const html = `
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; }
    .container { max-width: 500px; margin: 0 auto; padding: 20px; }
    .button { display: inline-block; padding: 12px 24px; background: #10b981; color: white; text-decoration: none; border-radius: 6px; }
    .group-name { font-weight: bold; color: #374151; }
    .footer { margin-top: 20px; font-size: 14px; color: #666; }
  </style>
</head>
<body>
  <div class="container">
    <h2>You've been invited to join a group</h2>
    <p><strong>${inviterEmail}</strong> has invited you to join <span class="group-name">"${groupName}"</span> on todu-fit.</p>
    <p>By joining, you'll be able to share dishes, meal plans, and shopping lists with the group.</p>
    <p><a href="${inviteLink}" class="button">Join Group</a></p>
    <p class="footer">
      This invitation will expire in ${expiresInHours} hours.<br>
      If you didn't expect this invitation, you can safely ignore this email.
    </p>
  </div>
</body>
</html>
`.trim()

  if (isDevMode) {
    console.log('\n=== GROUP INVITE EMAIL (DEV MODE) ===')
    console.log(`To: ${to}`)
    console.log(`Subject: ${subject}`)
    console.log(`Invite Link: ${inviteLink}`)
    console.log(`Group: ${groupName}`)
    console.log(`Inviter: ${inviterEmail}`)
    console.log('=====================================\n')
    return
  }

  if (!transporter) {
    throw new Error('Email transporter not configured')
  }

  await transporter.sendMail({
    from: `"${fromName}" <${fromEmail}>`,
    to,
    subject,
    text,
    html,
  })
}
