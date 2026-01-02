# iOS Application Architecture

## Overview

Native iOS app using SwiftUI and automerge-swift. Provides full CRUD capabilities with offline support and seamless sync.

**Repository:** `evcraddock/todufit-ios` (separate from Rust repos)

Since iOS uses Swift, it doesn't share code with the Rust projects. Instead, it implements the same Automerge document schema using `automerge-swift`, which is binary-compatible with `automerge-rs`.

## Tech Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| UI | SwiftUI | Declarative, modern iOS |
| Data sync | automerge-swift | Same CRDT as other clients |
| Local storage | SwiftData or Core Data | Query layer over Automerge |
| Auth storage | Keychain | Secure credential storage |
| Networking | URLSession | WebSocket for sync |
| Auth | ASAuthorizationController | Passkeys + magic links |

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          iOS App                                     │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                        SwiftUI Views                            │ │
│  │                                                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │ │
│  │  │   Meals     │  │   Dishes    │  │      Settings           │ │ │
│  │  │  Calendar   │  │   Browser   │  │   (Auth, Sync)          │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                              │                                      │
│                              ▼                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                      View Models                                │ │
│  │                  (ObservableObject)                            │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                              │                                      │
│                              ▼                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                     Data Layer                                  │ │
│  │                                                                │ │
│  │  ┌──────────────────┐  ┌──────────────────────────────────────┐ │ │
│  │  │   Repositories   │  │         Sync Engine                  │ │ │
│  │  │                  │  │                                      │ │ │
│  │  │  - DishRepo      │  │  - WebSocket client                  │ │ │
│  │  │  - MealPlanRepo  │  │  - Automerge sync protocol           │ │ │
│  │  │  - MealLogRepo   │  │  - Background sync                   │ │ │
│  │  └──────────────────┘  └──────────────────────────────────────┘ │ │
│  │           │                          │                         │ │
│  │           ▼                          ▼                         │ │
│  │  ┌──────────────────┐  ┌──────────────────────────────────────┐ │ │
│  │  │  SwiftData/Core  │  │       Automerge Documents            │ │ │
│  │  │   Data (query)   │  │         (source of truth)            │ │ │
│  │  └──────────────────┘  └──────────────────────────────────────┘ │ │
│  │           │                          │                         │ │
│  │           ▼                          ▼                         │ │
│  │  ┌──────────────────────────────────────────────────────────┐  │ │
│  │  │                    File System                            │  │ │
│  │  │  Documents/                                               │  │ │
│  │  │  ├── dishes.automerge                                    │  │ │
│  │  │  ├── mealplans.automerge                                 │  │ │
│  │  │  ├── meallogs.automerge                                  │  │ │
│  │  │  └── todufit.sqlite (query projection)                   │  │ │
│  │  └──────────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                        Auth                                     │ │
│  │                                                                │ │
│  │  ┌──────────────────┐  ┌──────────────────────────────────────┐ │ │
│  │  │     Keychain     │  │    ASAuthorizationController         │ │ │
│  │  │   (API key)      │  │    (Passkeys)                        │ │ │
│  │  └──────────────────┘  └──────────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────┘ │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
Todufit/
├── Todufit.xcodeproj
├── Todufit/
│   ├── TodufitApp.swift              # App entry point
│   ├── ContentView.swift             # Root navigation
│   │
│   ├── Models/
│   │   ├── Dish.swift
│   │   ├── MealPlan.swift
│   │   ├── MealLog.swift
│   │   └── MealType.swift
│   │
│   ├── ViewModels/
│   │   ├── MealsViewModel.swift
│   │   ├── DishesViewModel.swift
│   │   └── AuthViewModel.swift
│   │
│   ├── Views/
│   │   ├── Meals/
│   │   │   ├── MealCalendarView.swift
│   │   │   ├── DayView.swift
│   │   │   ├── LogMealView.swift
│   │   │   └── MealCardView.swift
│   │   ├── Dishes/
│   │   │   ├── DishListView.swift
│   │   │   ├── DishDetailView.swift
│   │   │   └── DishFormView.swift
│   │   ├── Auth/
│   │   │   ├── LoginView.swift
│   │   │   └── PasskeyButton.swift
│   │   └── Settings/
│   │       └── SettingsView.swift
│   │
│   ├── Data/
│   │   ├── Repositories/
│   │   │   ├── DishRepository.swift
│   │   │   ├── MealPlanRepository.swift
│   │   │   └── MealLogRepository.swift
│   │   ├── Sync/
│   │   │   ├── SyncEngine.swift
│   │   │   ├── AutomergeStorage.swift
│   │   │   └── WebSocketClient.swift
│   │   └── Projection/
│   │       └── SQLiteProjection.swift
│   │
│   ├── Auth/
│   │   ├── AuthManager.swift
│   │   ├── KeychainHelper.swift
│   │   └── PasskeyManager.swift
│   │
│   └── Resources/
│       └── Assets.xcassets
│
├── TodufitTests/
└── TodufitUITests/
```

## Data Flow

### Reading Data

```
View → ViewModel → Repository → SQLite (projected from Automerge)
```

SwiftData/Core Data provides fast queries. Automerge is source of truth but not directly queried.

### Writing Data

```
View → ViewModel → Repository → Automerge → Project to SQLite → Trigger Sync
```

```swift
class MealLogRepository: ObservableObject {
    private let automergeStorage: AutomergeStorage
    private let projection: SQLiteProjection
    private let syncEngine: SyncEngine
    
    func logMeal(_ input: LogMealInput) async throws {
        // 1. Write to Automerge document
        try await automergeStorage.withDocument(.mealLogs) { doc in
            let mealLog = MealLog(
                id: UUID(),
                date: input.date,
                mealType: input.mealType,
                dishes: input.dishes,
                notes: input.notes,
                createdBy: authManager.userId,
                createdAt: Date()
            )
            try doc.put(obj: .root, key: mealLog.id.uuidString, value: mealLog)
        }
        
        // 2. Project to SQLite for queries
        try await projection.projectMealLogs()
        
        // 3. Trigger background sync
        syncEngine.scheduleSync()
    }
}
```

### Sync Flow

```swift
class SyncEngine {
    private let webSocket: WebSocketClient
    private let storage: AutomergeStorage
    
    func sync() async throws {
        for docType in DocumentType.allCases {
            try await syncDocument(docType)
        }
    }
    
    private func syncDocument(_ docType: DocumentType) async throws {
        // Load local document
        var doc = try storage.load(docType) ?? Document()
        var syncState = SyncState()
        
        // Connect to server
        let url = buildSyncURL(docType: docType)
        try await webSocket.connect(to: url)
        
        // Generate initial message
        if let message = doc.generateSyncMessage(state: &syncState) {
            try await webSocket.send(message.encode())
        }
        
        // Sync loop
        for await data in webSocket.messages {
            let serverMessage = try SyncMessage.decode(data)
            try doc.receiveSyncMessage(state: &syncState, message: serverMessage)
            
            if let response = doc.generateSyncMessage(state: &syncState) {
                try await webSocket.send(response.encode())
            } else {
                break // Sync complete
            }
        }
        
        // Save updated document
        try storage.save(docType, document: doc)
    }
}
```

## Authentication

### Passkey Authentication (Preferred)

```swift
class PasskeyManager {
    private let authManager: AuthManager
    
    func signIn(email: String) async throws {
        // 1. Get challenge from server
        let startResponse = try await authManager.passkeyAuthStart(email: email)
        
        // 2. Create authorization request
        let challenge = Data(base64Encoded: startResponse.challenge)!
        let provider = ASAuthorizationPlatformPublicKeyCredentialProvider(
            relyingPartyIdentifier: "todufit.example.com"
        )
        
        let request = provider.createCredentialAssertionRequest(
            challenge: challenge
        )
        
        // 3. Present to user
        let controller = ASAuthorizationController(authorizationRequests: [request])
        controller.delegate = self
        controller.presentationContextProvider = self
        controller.performRequests()
        
        // Continues in delegate callback...
    }
    
    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard let credential = authorization.credential 
            as? ASAuthorizationPlatformPublicKeyCredentialAssertion else {
            return
        }
        
        Task {
            // 4. Send to server
            let response = try await authManager.passkeyAuthFinish(
                credentialID: credential.credentialID,
                clientDataJSON: credential.rawClientDataJSON,
                authenticatorData: credential.rawAuthenticatorData,
                signature: credential.signature
            )
            
            // 5. Store API key in Keychain
            try KeychainHelper.save(
                key: "api_key",
                value: response.apiKey
            )
        }
    }
}
```

### Magic Link Authentication (Fallback)

```swift
class AuthManager {
    func requestMagicLink(email: String) async throws {
        let callbackURL = "todufit://auth"  // Universal Link
        
        try await apiClient.post("/auth/login", body: [
            "email": email,
            "callback_url": callbackURL
        ])
    }
    
    // Called when app opens via Universal Link
    func handleMagicLinkCallback(url: URL) async throws {
        guard let token = URLComponents(url: url, resolvingAgainstBaseURL: false)?
            .queryItems?
            .first(where: { $0.name == "token" })?
            .value else {
            throw AuthError.invalidCallback
        }
        
        // Verify token with server
        let response: AuthResponse = try await apiClient.post("/auth/verify", body: [
            "token": token
        ])
        
        // Store API key
        try KeychainHelper.save(key: "api_key", value: response.apiKey)
        try KeychainHelper.save(key: "user_id", value: response.userId)
    }
}
```

### Universal Links Setup

**apple-app-site-association** (hosted on sync server):

```json
{
  "applinks": {
    "apps": [],
    "details": [
      {
        "appID": "TEAMID.com.example.todufit",
        "paths": ["/auth/*"]
      }
    ]
  }
}
```

**Xcode Entitlements:**

```xml
<key>com.apple.developer.associated-domains</key>
<array>
  <string>applinks:todufit.example.com</string>
</array>
```

## Keychain Storage

```swift
enum KeychainHelper {
    static func save(key: String, value: String) throws {
        let data = value.data(using: .utf8)!
        
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecAttrService as String: "com.example.todufit",
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlock
        ]
        
        // Delete existing
        SecItemDelete(query as CFDictionary)
        
        // Add new
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.saveFailed(status)
        }
    }
    
    static func load(key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecAttrService as String: "com.example.todufit",
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]
        
        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        
        guard status == errSecSuccess,
              let data = result as? Data,
              let value = String(data: data, encoding: .utf8) else {
            return nil
        }
        
        return value
    }
    
    static func delete(key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecAttrService as String: "com.example.todufit"
        ]
        SecItemDelete(query as CFDictionary)
    }
}
```

## Background Sync

```swift
// In TodufitApp.swift
@main
struct TodufitApp: App {
    @Environment(\.scenePhase) var scenePhase
    @StateObject var syncEngine = SyncEngine()
    
    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(syncEngine)
        }
        .onChange(of: scenePhase) { _, newPhase in
            switch newPhase {
            case .active:
                // Sync when app becomes active
                Task { try? await syncEngine.sync() }
            case .background:
                // Schedule background refresh
                scheduleBackgroundSync()
            default:
                break
            }
        }
        .backgroundTask(.appRefresh("sync")) {
            try? await syncEngine.sync()
        }
    }
    
    func scheduleBackgroundSync() {
        let request = BGAppRefreshTaskRequest(identifier: "sync")
        request.earliestBeginDate = Date(timeIntervalSinceNow: 15 * 60) // 15 min
        try? BGTaskScheduler.shared.submit(request)
    }
}
```

## Offline Support

The app works fully offline:

1. **Reads** come from SQLite projection (always available)
2. **Writes** go to local Automerge documents (always available)
3. **Sync** happens when network available

```swift
class SyncEngine {
    @Published var syncStatus: SyncStatus = .idle
    
    enum SyncStatus {
        case idle
        case syncing
        case offline
        case error(String)
    }
    
    func sync() async {
        guard NetworkMonitor.shared.isConnected else {
            syncStatus = .offline
            return
        }
        
        syncStatus = .syncing
        
        do {
            try await performSync()
            syncStatus = .idle
        } catch {
            syncStatus = .error(error.localizedDescription)
        }
    }
}
```

## Model Definitions

```swift
import Foundation
import Automerge

struct Dish: Codable, Identifiable {
    let id: UUID
    var name: String
    var ingredients: [Ingredient]
    var instructions: String
    var nutrients: [Nutrient]?
    var prepTime: Int?
    var cookTime: Int?
    var servings: Int?
    var tags: [String]
    var imageUrl: String?
    var sourceUrl: String?
    var createdBy: String
    var createdAt: Date
    var updatedAt: Date
}

struct Ingredient: Codable {
    var name: String
    var quantity: Double
    var unit: String
}

struct Nutrient: Codable {
    var name: String
    var amount: Double
    var unit: String
}

struct MealPlan: Codable, Identifiable {
    let id: UUID
    var date: Date
    var mealType: MealType
    var title: String
    var cook: String
    var dishes: [DishReference]
    var createdBy: String
    var createdAt: Date
    var updatedAt: Date
}

struct MealLog: Codable, Identifiable {
    let id: UUID
    var date: Date
    var mealType: MealType
    var mealPlanId: UUID?
    var dishes: [DishReference]
    var notes: String?
    var createdBy: String
    var createdAt: Date
}

struct DishReference: Codable {
    var dishId: UUID
    var servings: Double
}

enum MealType: String, Codable, CaseIterable {
    case breakfast
    case lunch
    case dinner
    case snack
}
```

## Automerge-Swift Integration

```swift
import Automerge

class AutomergeStorage {
    private let documentsURL: URL
    
    enum DocumentType: String, CaseIterable {
        case dishes
        case mealPlans = "mealplans"
        case mealLogs = "meallogs"
        
        var filename: String { "\(rawValue).automerge" }
    }
    
    func load(_ type: DocumentType) throws -> Document? {
        let url = documentsURL.appendingPathComponent(type.filename)
        guard FileManager.default.fileExists(atPath: url.path) else {
            return nil
        }
        let data = try Data(contentsOf: url)
        return try Document(data)
    }
    
    func save(_ type: DocumentType, document: Document) throws {
        let url = documentsURL.appendingPathComponent(type.filename)
        let data = document.save()
        try data.write(to: url)
    }
    
    func withDocument<T>(
        _ type: DocumentType,
        _ operation: (inout Document) throws -> T
    ) async throws -> T {
        var doc = try load(type) ?? Document()
        let result = try operation(&doc)
        try save(type, document: doc)
        return result
    }
}
```

## Testing

```swift
// Unit test for repository
class DishRepositoryTests: XCTestCase {
    var storage: AutomergeStorage!
    var repository: DishRepository!
    
    override func setUp() {
        let tempDir = FileManager.default.temporaryDirectory
            .appendingPathComponent(UUID().uuidString)
        try! FileManager.default.createDirectory(at: tempDir, withIntermediateDirectories: true)
        
        storage = AutomergeStorage(documentsURL: tempDir)
        repository = DishRepository(storage: storage)
    }
    
    func testCreateDish() async throws {
        let dish = try await repository.create(
            name: "Test Dish",
            ingredients: [],
            instructions: "Test instructions"
        )
        
        XCTAssertEqual(dish.name, "Test Dish")
        
        // Verify it was saved
        let loaded = try await repository.get(id: dish.id)
        XCTAssertEqual(loaded?.name, "Test Dish")
    }
}
```

## App Store Considerations

- **Privacy labels**: Disclose data collection (email for auth, meal data synced)
- **Keychain sharing**: If you have multiple apps, consider keychain access groups
- **Background modes**: Enable "Background fetch" for sync
- **Network permissions**: Add to Info.plist
