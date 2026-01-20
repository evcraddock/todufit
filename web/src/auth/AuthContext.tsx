import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react'
import { AuthState, AuthContextType } from './types'
import { fetchMe, logoutApi } from './api'

const AuthContext = createContext<AuthContextType | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [auth, setAuthState] = useState<AuthState | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  // Check session on mount
  const checkSession = useCallback(async () => {
    try {
      const authState = await fetchMe()
      setAuthState(authState)
      setError(null)
    } catch {
      // Not authenticated or session expired
      setAuthState(null)
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    checkSession()
  }, [checkSession])

  const refreshAuth = useCallback(async () => {
    setIsLoading(true)
    await checkSession()
  }, [checkSession])

  const login = useCallback(() => {
    // Navigate to login page
    window.location.href = '/login'
  }, [])

  const logout = useCallback(async () => {
    try {
      await logoutApi()
    } catch (e) {
      console.error('Logout error:', e)
    }
    setAuthState(null)
    setError(null)
    window.location.href = '/login'
  }, [])

  const clearError = useCallback(() => {
    setError(null)
  }, [])

  const value: AuthContextType = {
    auth,
    isAuthenticated: auth !== null,
    isLoading,
    error,
    login,
    logout,
    refreshAuth,
    clearError,
  }

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
