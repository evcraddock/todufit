import { useSearchParams, Link } from 'react-router-dom'

const ERROR_MESSAGES: Record<string, { title: string; message: string }> = {
  missing_token: {
    title: 'Invalid Link',
    message: 'This invitation link is missing required information.',
  },
  invalid: {
    title: 'Invalid Invitation',
    message: 'This invitation link is not valid. Please ask for a new invitation.',
  },
  expired: {
    title: 'Invitation Expired',
    message: 'This invitation has expired. Please ask the person who invited you to send a new invitation.',
  },
  already_used: {
    title: 'Already Accepted',
    message: 'This invitation has already been used. If you\'re having trouble accessing the group, please contact the person who invited you.',
  },
}

export function InviteError() {
  const [searchParams] = useSearchParams()
  const reason = searchParams.get('reason') || 'invalid'

  const errorInfo = ERROR_MESSAGES[reason] || ERROR_MESSAGES.invalid

  return (
    <div className="max-w-md mx-auto mt-12 p-6 bg-white dark:bg-gray-800 rounded-lg shadow-md">
      <div className="text-center">
        <div className="text-5xl mb-4">😕</div>
        <h2 className="text-2xl font-semibold mb-2 text-gray-900 dark:text-gray-100">
          {errorInfo.title}
        </h2>
        <p className="text-gray-600 dark:text-gray-400 mb-6">
          {errorInfo.message}
        </p>
        <Link
          to="/"
          className="inline-block px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          Go Home
        </Link>
      </div>
    </div>
  )
}
