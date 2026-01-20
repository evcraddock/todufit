import { useState, useEffect } from 'react'
import { useAuth } from './AuthContext'
import { setRootDocId, setCurrentGroup, SetRootDocIdError } from './api'

type SetupMode = 'choose' | 'create' | 'join' | 'invite'

interface IdentitySetupProps {
  onComplete: () => void
}

export function IdentitySetup({ onComplete }: IdentitySetupProps) {
  const { refreshAuth } = useAuth()
  const [mode, setMode] = useState<SetupMode>('choose')
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Invite flow state - tracks pending group invitation for new users.
  // When a user accepts an invite but has no identity, InviteSuccess stores
  // the group info in localStorage. We detect this and auto-create their identity.
  const [pendingGroupDocId, setPendingGroupDocId] = useState<string | null>(null)
  const [pendingGroupName, setPendingGroupName] = useState<string | null>(null)

  // Join flow state
  const [rootDocIdInput, setRootDocIdInput] = useState('')
  const [confirmChange, setConfirmChange] = useState(false)
  const [existingRootDocId, setExistingRootDocId] = useState<string | null>(null)

  // Create flow state
  const [groupName, setGroupName] = useState('')
  const [createdIdentityId, setCreatedIdentityId] = useState<string | null>(null)

  /**
   * Check for pending group invitation on mount.
   *
   * When a new user (no existing identity) accepts a group invite:
   * 1. InviteSuccess stores pendingJoinGroupDocId/Name in localStorage
   * 2. User is redirected to home, which shows IdentitySetup (no rootDocId)
   * 3. This effect detects the pending invite and switches to 'invite' mode
   * 4. The invite mode auto-creates an identity that joins the existing group
   */
  useEffect(() => {
    const groupDocId = localStorage.getItem('pendingJoinGroupDocId')
    const groupName = localStorage.getItem('pendingJoinGroupName')

    if (groupDocId && groupName) {
      setPendingGroupDocId(groupDocId)
      setPendingGroupName(groupName)
      setMode('invite')
    }
  }, [])

  // Trigger identity creation automatically when invite mode is detected
  useEffect(() => {
    if (mode === 'invite' && pendingGroupDocId && pendingGroupName && !isLoading && !createdIdentityId) {
      handleInviteSetup()
    }
  }, [mode, pendingGroupDocId, pendingGroupName])

  /**
   * Handle identity creation for invited users.
   *
   * Unlike the normal "create" flow which creates a new group, this flow:
   * 1. Creates a new identity for the user
   * 2. References the existing invited group (doesn't create new group docs)
   * 3. Stores pendingInviteGroup* keys for RepoContext to create the Automerge docs
   *
   * RepoContext will then create:
   * - Identity document with reference to the invited group
   * - Private meal logs document for this user
   * - Load shared docs (dishes, meal plans, shopping carts) from the invited group
   */
  const handleInviteSetup = async () => {
    if (!pendingGroupDocId || !pendingGroupName) return

    setIsLoading(true)
    setError(null)

    try {
      const { v4: uuidv4 } = await import('uuid')
      const bs58check = (await import('bs58check')).default

      // Generate a new identity ID (bs58check-encoded UUID)
      const uuidBytes = new Uint8Array(16)
      const uuid = uuidv4()
      const hexStr = uuid.replace(/-/g, '')
      for (let i = 0; i < 16; i++) {
        uuidBytes[i] = parseInt(hexStr.substr(i * 2, 2), 16)
      }
      const identityId = bs58check.encode(uuidBytes)

      // Generate a unique ID for this user's reference to the group
      const groupRefBytes = new Uint8Array(16)
      const groupRefUuid = uuidv4()
      const groupRefHexStr = groupRefUuid.replace(/-/g, '')
      for (let i = 0; i < 16; i++) {
        groupRefBytes[i] = parseInt(groupRefHexStr.substr(i * 2, 2), 16)
      }
      const groupRefId = bs58check.encode(groupRefBytes)

      // Save identity to server
      await setRootDocId(identityId)
      await setCurrentGroup(groupRefId)

      // Transfer invite data to pendingInvite* keys for RepoContext.
      // These keys signal that we're joining an EXISTING group, not creating a new one.
      localStorage.setItem('pendingInviteGroupDocId', pendingGroupDocId)
      localStorage.setItem('pendingInviteGroupName', pendingGroupName)
      localStorage.setItem('pendingInviteGroupRefId', groupRefId)

      // Clear the original keys from InviteSuccess
      localStorage.removeItem('pendingJoinGroupDocId')
      localStorage.removeItem('pendingJoinGroupName')

      setCreatedIdentityId(identityId)
      await refreshAuth()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create identity')
    } finally {
      setIsLoading(false)
    }
  }

  const handleJoinSubmit = async () => {
    if (!rootDocIdInput.trim()) {
      setError('Please enter a root doc ID')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      await setRootDocId(rootDocIdInput.trim(), confirmChange)
      await refreshAuth()
      onComplete()
    } catch (err) {
      const error = err as Error & SetRootDocIdError
      if (error.requires_confirmation) {
        setExistingRootDocId(error.current_root_doc_id || null)
        setError('You already have an identity set. Check the box to confirm changing it.')
      } else {
        setError(error.message || 'Failed to set root doc ID')
      }
    } finally {
      setIsLoading(false)
    }
  }

  const handleCreateSubmit = async () => {
    if (!groupName.trim()) {
      setError('Please enter a group name')
      return
    }

    setIsLoading(true)
    setError(null)

    try {
      // For now, we'll create a placeholder identity
      // The actual document creation happens in RepoContext when it initializes
      // We generate a random ID that will be used as the identity doc ID
      const { v4: uuidv4 } = await import('uuid')
      const bs58check = (await import('bs58check')).default

      // Generate a new identity ID (random UUID encoded as bs58check)
      const uuidBytes = new Uint8Array(16)
      const uuid = uuidv4()
      // Parse UUID into bytes
      const hexStr = uuid.replace(/-/g, '')
      for (let i = 0; i < 16; i++) {
        uuidBytes[i] = parseInt(hexStr.substr(i * 2, 2), 16)
      }
      const identityId = bs58check.encode(uuidBytes)

      // Save the root doc ID
      await setRootDocId(identityId)

      // Generate group ID
      const groupUuidBytes = new Uint8Array(16)
      const groupUuid = uuidv4()
      const groupHexStr = groupUuid.replace(/-/g, '')
      for (let i = 0; i < 16; i++) {
        groupUuidBytes[i] = parseInt(groupHexStr.substr(i * 2, 2), 16)
      }
      const groupId = bs58check.encode(groupUuidBytes)

      // Set as current group
      await setCurrentGroup(groupId)

      // Store the group name in localStorage for RepoContext to use when creating the docs
      localStorage.setItem('pendingGroupName', groupName.trim())
      localStorage.setItem('pendingGroupId', groupId)

      setCreatedIdentityId(identityId)
      await refreshAuth()
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create identity')
    } finally {
      setIsLoading(false)
    }
  }

  const handleCopyIdentityId = () => {
    if (createdIdentityId) {
      navigator.clipboard.writeText(createdIdentityId)
    }
  }

  // Show invite loading screen
  if (mode === 'invite' && !createdIdentityId) {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
        <div className="text-center">
          <div className="text-5xl mb-4">🔗</div>
          <h2 className="text-2xl font-semibold mb-2">
            {isLoading ? 'Setting up your account...' : 'Joining Group'}
          </h2>
          <p className="text-gray-600 mb-4">
            {isLoading
              ? `Creating your identity to join "${pendingGroupName}"...`
              : `You've been invited to join "${pendingGroupName}"`}
          </p>
          {error && (
            <div className="p-3 bg-red-100 border border-red-300 text-red-700 rounded-lg">
              {error}
            </div>
          )}
        </div>
      </div>
    )
  }

  // Show success screen after creating identity for invite flow
  if (createdIdentityId && mode === 'invite') {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
        <div className="text-center">
          <div className="text-5xl mb-4">🎉</div>
          <h2 className="text-2xl font-semibold mb-2">You've joined "{pendingGroupName}"!</h2>
          <p className="text-gray-600 mb-4">
            Your account is set up. You can now share dishes, meal plans, and shopping lists with this group.
          </p>
        </div>

        <button
          onClick={onComplete}
          className="w-full bg-green-600 text-white py-3 px-4 rounded-lg hover:bg-green-700 transition-colors"
        >
          Continue to App
        </button>
      </div>
    )
  }

  // Show success screen after creating identity (normal flow)
  if (createdIdentityId) {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
        <div className="text-center">
          <div className="text-5xl mb-4">🎉</div>
          <h2 className="text-2xl font-semibold mb-2">Identity Created!</h2>
          <p className="text-gray-600 mb-4">
            Your new identity has been created. Save this ID to sync with other devices:
          </p>
        </div>

        <div className="bg-gray-100 p-4 rounded-lg mb-4 break-all font-mono text-sm">
          {createdIdentityId}
        </div>

        <button
          onClick={handleCopyIdentityId}
          className="w-full bg-gray-200 text-gray-800 py-2 px-4 rounded-lg hover:bg-gray-300 mb-4 transition-colors"
        >
          Copy to Clipboard
        </button>

        <button
          onClick={onComplete}
          className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 transition-colors"
        >
          Continue to App
        </button>
      </div>
    )
  }

  // Choose mode screen
  if (mode === 'choose') {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
        <div className="text-center mb-6">
          <div className="text-5xl mb-4">🔗</div>
          <h2 className="text-2xl font-semibold mb-2">Set Up Your Identity</h2>
          <p className="text-gray-600">
            Your identity connects your data across devices.
          </p>
        </div>

        <div className="space-y-4">
          <button
            onClick={() => setMode('create')}
            className="w-full p-4 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors text-left"
          >
            <div className="font-semibold text-lg">Create New Identity</div>
            <div className="text-gray-600 text-sm">
              Start fresh with a new identity and group
            </div>
          </button>

          <button
            onClick={() => setMode('join')}
            className="w-full p-4 border-2 border-gray-200 rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors text-left"
          >
            <div className="font-semibold text-lg">Join Existing Identity</div>
            <div className="text-gray-600 text-sm">
              Enter an identity ID from another device
            </div>
          </button>
        </div>
      </div>
    )
  }

  // Create mode screen
  if (mode === 'create') {
    return (
      <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
        <button
          onClick={() => setMode('choose')}
          className="text-gray-600 hover:text-gray-800 mb-4"
        >
          ← Back
        </button>

        <div className="text-center mb-6">
          <div className="text-5xl mb-4">✨</div>
          <h2 className="text-2xl font-semibold mb-2">Create New Identity</h2>
          <p className="text-gray-600">
            Enter a name for your first group (e.g., "Family" or "Personal")
          </p>
        </div>

        {error && (
          <div className="mb-4 p-3 bg-red-100 border border-red-300 text-red-700 rounded-lg">
            {error}
          </div>
        )}

        <div className="mb-4">
          <label htmlFor="group-name" className="block text-sm font-medium text-gray-700 mb-1">
            Group Name
          </label>
          <input
            type="text"
            id="group-name"
            value={groupName}
            onChange={(e) => setGroupName(e.target.value)}
            placeholder="e.g., Family, Personal, Household"
            disabled={isLoading}
            className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50"
          />
        </div>

        <button
          onClick={handleCreateSubmit}
          disabled={isLoading || !groupName.trim()}
          className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {isLoading ? 'Creating...' : 'Create Identity'}
        </button>
      </div>
    )
  }

  // Join mode screen
  return (
    <div className="max-w-md mx-auto mt-12 p-6 bg-white rounded-lg shadow-md">
      <button
        onClick={() => {
          setMode('choose')
          setError(null)
          setConfirmChange(false)
          setExistingRootDocId(null)
        }}
        className="text-gray-600 hover:text-gray-800 mb-4"
      >
        ← Back
      </button>

      <div className="text-center mb-6">
        <div className="text-5xl mb-4">🔗</div>
        <h2 className="text-2xl font-semibold mb-2">Join Existing Identity</h2>
        <p className="text-gray-600">
          Enter the identity ID from another device or the CLI
        </p>
      </div>

      {error && (
        <div className="mb-4 p-3 bg-red-100 border border-red-300 text-red-700 rounded-lg">
          {error}
        </div>
      )}

      <div className="mb-4">
        <label htmlFor="root-doc-id" className="block text-sm font-medium text-gray-700 mb-1">
          Identity ID
        </label>
        <input
          type="text"
          id="root-doc-id"
          value={rootDocIdInput}
          onChange={(e) => setRootDocIdInput(e.target.value)}
          placeholder="e.g., 2vQoLBDgP8JEy..."
          disabled={isLoading}
          className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50 font-mono text-sm"
        />
      </div>

      {existingRootDocId && (
        <div className="mb-4 p-3 bg-yellow-100 border border-yellow-300 rounded-lg">
          <p className="text-yellow-800 text-sm mb-2">
            Current identity: <span className="font-mono">{existingRootDocId}</span>
          </p>
          <label className="flex items-center gap-2 text-yellow-800">
            <input
              type="checkbox"
              checked={confirmChange}
              onChange={(e) => setConfirmChange(e.target.checked)}
              className="rounded"
            />
            <span className="text-sm">I want to change my identity</span>
          </label>
        </div>
      )}

      <button
        onClick={handleJoinSubmit}
        disabled={isLoading || !rootDocIdInput.trim()}
        className="w-full bg-blue-600 text-white py-3 px-4 rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {isLoading ? 'Joining...' : 'Join Identity'}
      </button>
    </div>
  )
}
