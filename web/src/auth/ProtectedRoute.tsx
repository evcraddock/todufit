import { ReactNode, useState } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { useAuth } from './AuthContext'
import { IdentitySetup } from './IdentitySetup'

interface ProtectedRouteProps {
  children: ReactNode
  requireIdentity?: boolean
}

export function ProtectedRoute({ children, requireIdentity = true }: ProtectedRouteProps) {
  const { isAuthenticated, isLoading, auth, refreshAuth } = useAuth()
  const location = useLocation()
  const [setupComplete, setSetupComplete] = useState(false)

  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[50vh]">
        <div className="text-gray-500">Loading...</div>
      </div>
    )
  }

  if (!isAuthenticated) {
    // Redirect to login, preserving the intended destination
    return <Navigate to="/login" state={{ from: location }} replace />
  }

  // Check if identity setup is required and not yet complete
  if (requireIdentity && !auth?.rootDocId && !setupComplete) {
    return (
      <IdentitySetup
        onComplete={async () => {
          await refreshAuth()
          setSetupComplete(true)
        }}
      />
    )
  }

  return <>{children}</>
}
