/**
 * Calendar subscription API client.
 */

interface CalendarTokenResponse {
  token: string | null
  subscription_url?: string
  http_url?: string
  created_at?: string
}

interface CreateTokenResponse {
  token: string
  subscription_url: string
  http_url: string
  created_at: string
  regenerated: boolean
}

/**
 * Get the current calendar token for a group.
 */
export async function getCalendarToken(groupDocId: string): Promise<CalendarTokenResponse> {
  const response = await fetch(`/api/calendar/token?group_doc_id=${encodeURIComponent(groupDocId)}`, {
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Failed to get calendar token')
  }

  return response.json()
}

/**
 * Create or regenerate a calendar token for a group.
 */
export async function createCalendarToken(
  groupDocId: string,
  regenerate = false
): Promise<CreateTokenResponse> {
  const response = await fetch('/api/calendar/token', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify({ group_doc_id: groupDocId, regenerate }),
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Failed to create calendar token')
  }

  return response.json()
}

/**
 * Revoke the calendar token for a group.
 */
export async function revokeCalendarToken(groupDocId: string): Promise<void> {
  const response = await fetch(`/api/calendar/token?group_doc_id=${encodeURIComponent(groupDocId)}`, {
    method: 'DELETE',
    credentials: 'include',
  })

  if (!response.ok) {
    const error = await response.json()
    throw new Error(error.error || 'Failed to revoke calendar token')
  }
}
