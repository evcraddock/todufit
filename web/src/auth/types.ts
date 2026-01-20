export interface PasskeyInfo {
  id: string
  name: string | null
  created_at: string
  last_used_at: string | null
}

export interface AuthState {
  userId: string
  email: string
  rootDocId: string | null
  currentGroupId: string | null
  isAdmin: boolean
  passkeys: PasskeyInfo[]
}

export interface AuthContextType {
  auth: AuthState | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
  login: () => void
  logout: () => Promise<void>
  refreshAuth: () => Promise<void>
  clearError: () => void
}
