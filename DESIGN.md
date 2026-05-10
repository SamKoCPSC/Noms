# Noms — Product & Software Design Brainstorm

> A Recipe Management and Sharing Application

---

## Table of Contents
1. [Product Vision](#product-vision)
2. [Target Users](#target-users)
3. [Core Features](#core-features)
4. [Feature Deep Dives](#feature-deep-dives)
5. [User Flows](#user-flows)
6. [Technical Architecture](#technical-architecture)
7. [Data Model](#data-model)
8. [UI/UX Design Principles](#uiux-design-principles)
9. [Open Questions & Decisions](#open-questions--decisions)
10. [Competitive Landscape](#competitive-landscape)

---

## Product Vision

Noms is a full-stack, community-driven recipe management and sharing platform. It combines personal recipe organization with social discovery and collaboration features. Users can create, save, version, and share their recipes in a living ecosystem where others can discover them, interact with creators, copy or fork recipes to make their own variations, and build upon the work of others.

**Core metaphor:** Think "GitHub for Recipes" — personal recipe management meets community-driven collaboration with full version history.

**Key differentiators from existing tools (Paprika, etc.):**
- **Version history** on every recipe — track how your grandmother's lasagna evolved over years
- **Fork model** — fork someone's recipe, modify it to your taste, maintain your own branch while attributing the original
- **Community interaction** — comment on recipes, follow creators, discover through a social feed
- **Dual audience** — great for everyday home cooks, with advanced features for power users and businesses

---

## Target Users

### Primary: Everyday Home Cooks
- Wants to save recipes from the web without losing them in browser bookmarks
- Wants a clean, organized personal cookbook they can actually use while cooking
- Enjoys discovering new recipes through a trusted community
- Shares family recipes with friends and relatives

### Secondary: Power Users & Food Enthusiasts
- Maintains extensive recipe collections with detailed notes and variations
- Actively contributes to the community — publishes, responds to comments, forks and remixes
- Tracks version history of beloved family recipes across generations

### Tertiary: Businesses (Future)
- Restaurants or food bloggers who want a public-facing recipe portfolio
- Meal planning services or catering businesses managing large recipe libraries
- Brand accounts with curated collections

---

## Core Features

Features are listed in linear implementation priority — foundational work first, dependent features later. Each item builds on what came before it.

### Phase 1: Foundation (Users & Content)
- [ ] User authentication & profiles
- [ ] Create recipes from scratch (title, ingredients, steps, photos, metadata)
- [ ] **Recipe Change History Tracking:** View a timeline of past versions with highlighted diffs (added/removed ingredients) and restore previous versions instantly. *(Core value prop; must exist before forking makes sense)*

### Phase 2: Content Growth & Organization
- [ ] Import recipes from URLs (scrape structured data from recipe blog pages)
- [ ] Search and filter personal recipe library
- [ ] **File System Organization:** Infinite nesting of collections/folders to organize recipes hierarchically.

### Phase 3: Public Exposure & Social Basics
- [ ] Public profiles showcasing a user's published recipes
- [ ] Share individual recipes via link
- [ ] Like/favorite recipes from others
- [ ] Top-level comments on recipes (flat conversations)
- [ ] **Structured Tag System + Dietary Filters:** Standardized, platform-wide tags (`#Vegan`, `#GlutenFree`, `#Dinner`, `#30min`) alongside freeform collections. Powers faceted search and instant dietary filtering without relying solely on text parsing.

### Phase 4: Community & Collaboration
- [ ] Follow other users to see their new recipes in a feed
- [ ] **Fork a recipe** — copy someone's recipe into your own library, modify freely, with attribution to the original *(Depends on versioning being solid)*
- [ ] **Recipe Variations / Personal Branching:** Test private modifications ("Spicy Version", "Vegan Swap") under a single recipe before deciding whether to publish it as a full public fork. Like Git branches for recipes — keeps your main recipe clean while you experiment. *(Depends on versioning & forking)*
- [ ] **Notification System:** In-app and email alerts when someone forks your recipe, comments on it, or starts following you. Essential for social retention once community features launch.
- [ ] Recipe Scaling UI — dynamically adjust serving sizes; automatically calculates new ingredient quantities while preserving formatting notes (e.g., "pinch of salt").

### Phase 5: Search & Conversation Depth
- [ ] **Nested Comment Threads:** Threaded replies under top-level comments with UI indentation, collapse/expand functionality, and support for deep conversation trees. *(Depends on flat comments existing)*
- [ ] **Search Autocomplete:** Type-ahead suggestions in the search bar powered by `pg_trgm` for instant infix matching and fuzzy similarity — "gar" suggests "Garlic Bread", "Garam Masala Chicken", etc. (sub-50ms via GIN trigram index). *(Most valuable once recipe volume is high enough to warrant it)*

### Phase 6: Daily Utility & Planning
- [ ] **Ingredient Discovery & Pantry Management:** Maintain a lightweight pantry inventory of common staples. Query "Show me recipes I can make with what I have" powered by existing ingredient indexing and trigram matching.
- [ ] **Meal Planner Calendar + Shopping List Aggregator:** Drag recipes onto calendar days to plan meals, then auto-aggregate and de-duplicate ingredients across the week into a single actionable shopping list. *(Depends on having robust recipe scaling)*
- [ ] Image-based recipe capture (take a photo of a printed recipe, AI extracts structured data)
- [ ] Nutritional information estimation
- [ ] Print-friendly / PDF export for individual recipes or collections
- [ ] Business accounts — public-facing portfolio pages, analytics on recipe views/saves
- [ ] Recipe collaboration — co-edit a recipe with another user in real time (like Google Docs for recipes)
- [ ] Recipe "pull requests" — suggest edits to someone else's recipe they can accept/reject

---

---

## Feature Deep Dives

<!-- This section explores specific features in detail as we design them -->

### 1. Recipe Import Pipeline (URL Parsing)

**Problem:** Users want to save recipes from food blogs with minimal friction. The best experience is pasting a URL and getting a structured, editable recipe draft instantly.

#### Approach: Two-Stage Import Flow
```
User pastes URL → Backend fetches HTML → Extracts schema.org/JSON-LD data → 
Fallback to heuristic parsing if no structured data → Returns parsed JSON to frontend → 
User reviews/edits in form → Saves as Recipe v1
```

**Parsing Strategy (Rust):**
1. **Primary:** Extract `Recipe` schema from `<script type="application/ld+json">` blocks (JSON-LD). Most modern recipe sites use this for SEO.
2. **Secondary:** Fallback to HTML parsing using the `select` crate with CSS selectors targeting common recipe markup patterns (`<span class="recipe-ingredient">`, etc.)
3. **Tertiary:** If all else fails, return raw page text and let user manually copy-paste

**Key Rust crates to evaluate:**
- `reqwest` — HTTP client for fetching URLs (with configurable User-Agent headers)
- `scraper` or `select` — HTML parsing with CSS selector support
- `serde_json` — JSON-LD extraction and deserialization
- Potential: `recipe-parser-rs` if a community crate exists

**Edge Cases to Handle:**
- Sites that block automated requests (Cloudflare protection, robots.txt)
- Multi-page recipes requiring pagination
- Recipes with mixed measurement systems (metric vs imperial)
- Images embedded in `<img>` tags vs lazy-loaded via JavaScript
- Ad-heavy pages where recipe content is buried

**Parsed Data Structure:**
```rust
struct ParsedRecipe {
    title: String,
    description: Option<String>,
    author: Option<String>,
    image_urls: Vec<Url>,
    prep_time: Option<Duration>,
    cook_time: Option<Duration>,
    total_time: Option<Duration>,
    servings: Option<u32>,
    ingredients: Vec<ParsedIngredient>,
    instructions: Vec<ParsedStep>,
    cuisine: Option<String>,
    keywords: Vec<String>,
}

struct ParsedIngredient {
    text: String,          // Raw text from the page
    name: Option<String>,  // Extracted ingredient name
    quantity: Option<f64>, // Numeric amount
    unit: Option<String>,  // cups, tbsp, grams, etc.
}

struct ParsedStep {
    text: String,
    order: u32,
}
```

**Open Question #2 Resolution Direction:** The two-stage approach (auto-parse → manual review/edit) gives users the best of both worlds — convenience for well-structured sites, full control when parsing fails or needs adjustment. This should be our default strategy.

---

### 2. Authentication & Session Management

**Philosophy:** Delegate identity management to Google OAuth — we're building a recipe platform, not an auth provider. This eliminates password storage, breach risk, MFA implementation, and account recovery headaches. Users get a single click "Sign in with Google" experience that's familiar and trusted.

#### Strategy: Server-Side Sessions over JWTs

**Why sessions instead of JWTs?**
- **Revocation:** We can instantly invalidate a session (important for logout, security incidents) without token rotation complexity
- **Storage:** Server-side state means we control the lifecycle; no client-side secret management
- **Size:** Session ID is small (~32 bytes UUID); no bloated JWT payloads in every request header
- **Security:** HTTP-only cookies prevent XSS token theft; SameSite=Strict prevents CSRF

**Trade-off acknowledged:** Slightly more database reads per request to validate sessions, but PostgreSQL handles this trivially at our scale. The security benefits far outweigh the negligible performance cost.

#### OAuth 2.0 Flow (Google)

```
┌──────────┐     ┌──────────────┐     ┌─────────────┐     ┌──────────┐
│   User   │     │  Dioxus UI   │     │ Axum Backend │     │  Google  │
└────┬─────┘     └──────┬───────┘     └──────┬──────┘     └────┬─────┘
     │                  │                     │                 │
     │ "Sign in with    │                     │                 │
     │ Google" click    │                     │                 │
     ├─────────────────►│                     │                 │
     │                  │ Redirect to         │                 │
     │                  │ /auth/google/start  │                 │
     │                  ├────────────────────►│                 │
     │                  │                     │ Generate        │
     │                  │                     │ auth state      │
     │                  │                     │ (CSRF nonce)    │
     │                  │                     ├────────────────►│
     │                  │                     │ Google OAuth URL│
     │ Browser redirect │                     │                 │
     │├─────────────────┤                     │                 │
     │► Google Consent  │                     │                 │
     │◄ Authorization   │                     │                 │
     │ Code + State     │                     │                 │
     ├─────────────────►│ /auth/google/callback│                │
     │                  ├────────────────────►│                 │
     │                  │                     │ Exchange code   │
     │                  │                     │ for tokens      │
     │                  │                     ├────────────────►│
     │                  │                     │ Access + ID     │
     │                  │                     │ tokens          │
     │                  │                     │◄────────────────┤
     │                  │                     │                 │
     │                  │                     │ Verify JWT      │
     │                  │                     │ (google_sign_in│
     │                  │                     │  library)       │
     │                  │                     │                 │
     │                  │                     │ Create/lookup   │
     │                  │                     │ User record     │
     │                  │                     │                 │
     │                  │                     │ Generate        │
     │                  │                     │ session UUID    │
     │                  │                     │ Insert into DB  │
     │                  │                     │                 │
     │ Set HTTP-only   │◄────────────────────┤                 │
     │ cookie + redirect│                     │                 │
     │ to /dashboard    │                     │                 │
     │◄─────────────────┤                     │                 │
     │                  │                     │                 │
```

**State parameter (critical security):** Each auth request generates a random UUID stored server-side. Google echoes it back in the callback — we verify it matches to prevent CSRF attacks during OAuth flow.

#### Rust Crate Ecosystem

| Crate | Purpose | Why It's Right |
|-------|---------|----------------|
| `oauth2` | Standard OAuth 2.0 client | Well-maintained, async-compatible, Google provider support built-in |
| `google-signin` (or manual verification) | Verify Google ID tokens | Validates JWT signature against Google's public keys, checks audience/issuer/expiry |
| `axum-extra` | Cookie management | Ergonomic HTTP-only cookie helpers integrated with Axum extractors |
| `jsonwebtoken` | Parse and verify Google ID tokens | If not using a higher-level library like `google-signin` |
| `uuid` | Session IDs, user IDs | Secure random UUID generation |

#### Database Schema Additions

```sql
-- Extended User table for OAuth
ALTER TABLE users ADD COLUMN IF NOT EXISTS google_sub VARCHAR(255) UNIQUE;  -- Google's unique user ID
ALTER TABLE users ADD COLUMN IF NOT EXISTS email_verified BOOLEAN DEFAULT TRUE;  -- Google emails are verified by default
ALTER TABLE users ADD COLUMN IF NOT EXISTS last_sign_in_at TIMESTAMPTZ;

-- Session management (server-side)
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,  -- Absolute expiry (e.g., 30 days from creation)
    last_active_at TIMESTAMPTZ DEFAULT NOW(),  -- Rolling "remember me" window
    ip_address INET,  -- Optional: track login IP for security auditing
    user_agent TEXT,  -- Optional: detect unusual client changes

    CONSTRAINT valid_expiry CHECK (expires_at > created_at)
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);  -- For cleanup job

-- Auth state store for OAuth CSRF protection (short-lived, ~10 min TTL)
CREATE TABLE auth_states (
    id VARCHAR(64) PRIMARY KEY,  -- The random state string sent to Google
    redirect_uri TEXT NOT NULL,  -- Where user intended to go after login
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT state_expiry CHECK (NOW() < created_at + INTERVAL '10 minutes')
);

-- Periodic cleanup of expired states and sessions
CREATE OR REPLACE FUNCTION cleanup_expired_auth_data()
RETURNS void AS $$
BEGIN
    DELETE FROM auth_states WHERE created_at < NOW() - INTERVAL '10 minutes';
    DELETE FROM sessions WHERE expires_at < NOW();
END;
$$ LANGUAGE plpgsql;

-- Run via pg_cron or application-level cron job every hour
```

#### Session Lifecycle & Cookie Configuration

**Cookie attributes:**
```rust
// Pseudocode for cookie configuration
let session_cookie = Cookie::build(("session_id", session_uuid.to_string()))
    .http_only(true)     // JavaScript cannot read this — prevents XSS token theft
    .secure(true)        // Only sent over HTTPS (Railway enforces this anyway)
    .same_site(SameSite::Strict)  // Prevents cross-site request forgery
    .path("/")           // Available to all routes
    .max_age(Duration::from_secs(30 * 24 * 60 * 60))  // 30 days
    .finish();
```

**Rolling expiry strategy:**
- **Absolute expiry:** Session expires after 30 days regardless of activity (security boundary)
- **Rolling refresh:** Each request extends `last_active_at`; if gap between requests exceeds 7 days, require re-authentication
- **Logout:** Immediate deletion from sessions table + cookie clearance on client

#### Dioxus Integration Pattern

**Server-side authentication middleware in Axum:**
```rust
// Extract authenticated user from session cookie
struct AuthenticatedUser {
    pub id: Uuid,
    pub username: String,
    // ... other profile data
}

impl FromRequestParts for AuthenticatedUser {
    async fn from_request_parts(parts: &mut Parts, _state: &RequestPartsExt) -> Result<Self> {
        let session_cookie = extract_session_id(parts)?;
        let session = validate_session_in_db(session_cookie).await?;
        Ok(UserRecord::from_db(session.user_id))
    }
}

// Usage in routes
async fn create_recipe(user: AuthenticatedUser, body: RecipeDraft) -> impl IntoResponse {
    // user.id is the recipe owner — no token parsing needed
}
```

**Client-side auth state in Dioxus:**
```rust
// Global auth context for the UI
pub struct AuthContext {
    pub current_user: Option<UserProfile>,  // Set during SSR or after login redirect
    pub is_authenticated: bool,
}

// Components check this context to render appropriately:
// - Authenticated user sees their dashboard, edit buttons, etc.
// - Unauthenticated visitor sees public recipes with "Sign in to fork" prompts
```

**SSR Consideration:** During server-side rendering, Dioxus has access to the session cookie directly (it's part of the HTTP request). This means:
- SSR can render personalized content immediately (no flash of unauthenticated state)
- No client-side auth check race condition — the page is already aware of login status
- Clean separation: Axum validates → Dioxus renders with user context → hydration connects interactivity

#### Security Checklist

| Concern | Mitigation | Status |
|---------|------------|--------|
| CSRF on OAuth callback | State parameter verification | ✅ Designed in |
| XSS token theft | HTTP-only cookies (JS cannot access) | ✅ Designed in |
| Session hijacking | Secure + SameSite cookie flags, IP tracking for anomalies | ✅ Designed in |
| Stale sessions | Expiration cleanup job (pg_cron or app-level cron) | ✅ Designed in |
| Account takeover via Google compromise | We trust Google's auth — if their account is compromised, user re-authenticates with new credentials. Consider adding email confirmation for sensitive actions as future hardening. | ⚠️ Future consideration |
| Brute force login | N/A — Google handles rate limiting on their side | ✅ Delegated to Google |
| Password storage | None! No passwords in our system at all | ✅ Eliminated entirely |

#### Multi-Provider Authentication Strategy

**Decision: Google, Apple, and GitHub Sign-In** — covers the vast majority of general consumers while explicitly welcoming our developer audience for open-source contribution.

##### Provider Analysis & Selection Rationale

| Provider | User Coverage | Pros | Cons | Verdict |
|----------|--------------|------|------|---------|
| **Google** | ~90% (most people have a Gmail account) | Dominant web/mobile, trusted brand, easy setup, rich profile data (name, verified email, avatar), single-click login on Chrome/Android | Privacy concerns in some regions (EU/GDPR users may prefer alternatives) | ✅ Include — essential baseline |
| **Apple** | ~50% (iOS/macOS ecosystem) | Required for iOS native app later, growing privacy-focused user base, clean UI guidelines from Apple, Sign In with Apple button expected by Apple users | Truncated emails by default (privacy feature), limited profile data (no avatar URL), "Sign in with Apple" button styling must follow Apple's HIG exactly | ✅ Include — table stakes for modern web apps |
| **Facebook** | ~75% globally | Massive user base, especially older demographics and international markets | Declining trust among younger users, associated with spam/bot accounts, complex API changes over years, brand misalignment (food community ≠ Facebook's reputation) | ❌ Skip — doesn't align with product positioning |
| **GitHub** | <10% of general public, ~90% of contributors | Essential for open-source community, aligns with "GitHub for Recipes" metaphor, zero friction for devs | Irrelevant to non-technical users (but they won't notice or care) | ✅ Include — critical for developer/community engagement |

##### Why Google + Apple + GitHub is the Right Combo

1. **Virtually 100% coverage:** Anyone with a smartphone has either a Google or Apple account
2. **Complementary strengths:** Google covers web/Android; Apple covers iOS privacy-conscious users
3. **Same architectural pattern:** Both use standard OAuth 2.0 / OpenID Connect — identical flow, just different endpoints and scopes
4. **Future-proofing:** Apple Sign In is required if we ever publish a native iOS app to the App Store
5. **Manageable complexity:** Two providers = double the configuration but not double the code (shared auth infrastructure)

##### Updated Database Schema for Multiple Providers

```sql
-- Replace single google_sub column with provider-agnostic design
ALTER TABLE users DROP COLUMN IF EXISTS google_sub;

CREATE TABLE oauth_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(20) NOT NULL CHECK (provider IN ('google', 'apple', 'github')),
    provider_user_id VARCHAR(255) NOT NULL,  -- Google's "sub" or Apple's user ID
    email VARCHAR(255),                      -- Provider-verified email at time of linking
    email_verified BOOLEAN DEFAULT FALSE,
    profile_data JSONB,                      -- Flexible: name, avatar_url, locale, etc.
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(provider, provider_user_id)  -- One account per provider per user
);

CREATE INDEX idx_oauth_accounts_email ON oauth_accounts(email);

-- This design supports:
-- 1. Single sign-in method (most common): User has one row in oauth_accounts
-- 2. Multiple methods linked to same account: User has multiple rows, all pointing to same user_id
--    Example: Signed up with Google, later added Apple as alternative login method
```

##### Identity Strategy: Enforced Automatic Account Linking

**Decision:** We will treat **email as the universal source of truth** for identity unification across providers. When a user logs in with Provider B and their verified email matches an existing account linked to Provider A, we automatically link them into a single Noms account.

#### Why Seamless Linking?

1.  **Cross-Device Fluidity:** This is the primary driver. Users expect to access their content regardless of the device or browser they are using today. If Alice logs in with Google on her Android phone and then tries to view recipes on her iPad via Apple Sign-In, she *must* see the same library. Fragmentation destroys trust and retention immediately.
2.  **Frictionless Onboarding:** Users often have multiple accounts (Google/Apple) but one email identity. Allowing them to "just click" whatever sign-in button is convenient without worrying about creating a duplicate profile creates a superior user experience.
3.  **Data Integrity:** Consolidating activity (follows, forks, comments) under one identity makes the social graph cleaner and more meaningful than having "Ghost Accounts" for the same person across different devices.

#### The Linking Logic

The auth callback handler follows this specific priority order:

1.  **Check Provider ID (`provider_user_id`):**
    *   Does a row exist in `oauth_accounts` with this exact provider + ID?
    *   **Yes:** This is an existing user logging in normally. Create session -> Done.
    *   **No:** Proceed to Step 2 (New login attempt).

2.  **Check Email (`email`):**
    *   Does a row exist in `oauth_accounts` for this email address (associated with *any* provider)?
    *   **Yes (Merge/Link):** The user already exists on the platform under a different provider. We insert a new row into `oauth_accounts` linking their current Provider ID to that existing `user_id`. Create session -> Done. 
        *   *Result:* User now has one account accessible via both Google and Apple.
    *   **No (New Account):** No user exists with this email. Create a new `users` record and link the provider. Create session -> Done.

#### Handling Edge Cases

**Apple's Truncated Emails:**
*   **First Login:** When a user logs in with Apple for the very first time, Apple provides their real email address (e.g., `alice@gmail.com`). Our logic matches this against Alice's existing Google account (`alice@gmail.com`) and links them seamlessly.
*   **Subsequent Logins:** On future logins, Apple returns a relay email (`xyz@privaterelay.appleid.com`). However, since Step 1 (Provider ID check) will find the existing record from the first login, we never reach Step 2. The truncated email is irrelevant for returning users.

**Shared Family Emails (The "Mom's iPad" Problem):**
*   *Scenario:* A child uses `family@gmail.com` on their device via Google. They try to sign in with Apple using a different email but accidentally use the shared family email, or vice versa.
*   *Mitigation:* This is an inherent edge case of identity systems. Initially, we trust the provider-verified email as the source of truth. If two distinct people share one verified Google account and try to use Noms separately via Apple later, they might get merged. We accept this risk for now in favor of the 95% seamless experience for normal users. We can add manual "Unmerge" tools in Settings if support tickets arise.

#### Updated Auth Logic (Simplified)

```rust
async fn handle_oauth_callback(provider: &str, provider_user_id: String, 
                                profile_data: ProfileInfo) -> Result<Session> {
    // Step 1: Check if this specific provider+ID exists already
    let existing = oauth_accounts.find_by_provider_and_id(provider, &provider_user_id).await;
    
    match existing {
        Some(account) => {
            // Existing user logging in with their usual method -> Create session
            update_last_used_at(&account.id).await?;
            create_session(account.user_id).await
        }
        
        None => {
            // Step 2: New login for this provider. Check if email already exists on another provider.
            
            let maybe_existing_user = oauth_accounts
                .find_by_email(&profile_data.email) 
                .await;

            match maybe_existing_user {
                Some(existing_account) => {
                    // MERGE: Email matches an existing user! Link this new provider to them.
                    oauth_accounts.insert(
                        user_id: existing_account.user_id,
                        provider, 
                        provider_user_id,
                        email: profile_data.email
                    ).await?;

                    create_session(existing_account.user_id).await
                }
                
                None => {
                    // NEW USER: Create a completely fresh identity
                    
                    let suggested_username = generate_unique_username(&profile_data);
                    
                    let new_user = users.insert(
                        username: suggested_username,
                        display_name: profile_data.name,
                        avatar_url: profile_data.avatar
                    ).await?;

                    oauth_accounts.insert(
                        user_id: new_user.id, 
                        provider, 
                        provider_user_id,
                        email: profile_data.email
                    ).await?;

                    create_session(new_user.id).await
                }
            }
        }
    }
}
```

**Key Takeaway:** This approach prioritizes the user's seamless experience over architectural minimalism. By treating email as the bridge, we ensure that a user's Noms identity travels with them across devices and platforms naturally.

##### Provider-Specific Quirks to Handle

**Apple Sign In:**
```rust
// Apple sends a "real" email only on FIRST sign-in. 
// After that, it sends a relay email: xyz@privaterelay.appleid.com
// We MUST store the first real email and use it for future lookups.

struct AppleProfile {
    sub: String,          // Apple's unique user ID (persistent)
    email: Option<String>, // Real email on FIRST login only!
    is_private_email: bool, // True if using relay email
    name: Option<AppleName>, // Optional: given_name + family_name
    
    // CRITICAL: If this is the first time we see this Apple user_id,
    // and they provided a real email, store it permanently.
    // Future logins will only give us the relay email.
}

// Flow for Apple specifically:
// 1. User clicks "Sign in with Apple" → browser shows native Apple consent sheet
// 2. On FIRST sign-in ever: Apple gives us their REAL email + name (if user allows)
// 3. On ALL subsequent sign-ins: Apple gives us a relay email (xyz@privaterelay.appleid.com)
//    and NO name data
// 
// Our system must recognize that the same `sub` value = same user, regardless of email changes.
```

**Google Sign In:**
- Consistent behavior across all logins (always gives full email + profile data)
- No special handling needed beyond standard OAuth flow

##### Updated Rust Crate Dependencies

| Crate | Purpose | Notes |
|-------|---------|-------|
| `oauth2` | Standard OAuth 2.0 client | Works identically for Google and Apple — just different authorization/token URLs |
| `jsonwebtoken` | Parse/verify ID tokens (Google) + JWT validation (Apple) | Apple returns a signed JWT with user info; we verify against Apple's JWKS endpoint |
| `axum-extra` | Cookie management | Unchanged from single-provider design |
| `uuid` | Session IDs, auth state | Unchanged |

##### Updated Auth UI Flow

```html
<!-- Sign-in page shows both buttons -->
<div class="auth-buttons">
    <button class="google-signin-btn">
        <!-- Google's branded button (follows Material Design guidelines) -->
        Sign in with Google
    </button>
    
    <button class="apple-signin-btn">
        <!-- Apple requires specific styling: black/dark button, white logo 
             Must use Apple-provided JS SDK or follow their HIG exactly -->
        Sign in with Apple
    </button>
</div>

<!-- After login, profile settings page allows adding additional providers -->
<div class="linked-accounts">
    <h3>Connected Accounts</h3>
    
    <!-- Google: Connected → Show as linked, allow unlinking (if other provider exists) -->
    <div class="account-row connected">
        <img src="google-logo.svg" />
        <span>user@gmail.com</span>
        <button onclick="unlink('google')">Remove</button>
    </div>
    
    <!-- Apple: Not Connected → Show "Connect" button -->
    <div class="account-row not-connected">
        <img src="apple-logo.svg" />
        <span>Not connected</span>
        <button onclick="linkApple()">Connect</button>
    </div>
    
    <!-- Important: Can't unlink if it's the ONLY linked provider (would lock user out) -->
</div>
```

##### Updated Security Checklist for Multi-Provider

| Concern | Mitigation | Status |
|---------|------------|--------|
| Provider-specific vulnerabilities | We trust both Google and Apple's auth infrastructure. Each has extensive security teams monitoring for issues. | ✅ Delegated to providers |
| Email collision attacks (malicious user claims same email across providers) | We require provider-verified emails only (both Google and Apple guarantee verified). No self-reported emails allowed during signup. | ✅ Designed in via OAuth-only strategy |
| Account hijacking via provider compromise | If a provider is compromised, user re-authenticates with remaining linked provider(s). Multiple linked accounts = redundancy. | ✅ Mitigated by design |
| Truncated email causing lookup failures (Apple) | We use `provider_user_id` as the primary identifier, NOT email. Email is only for linking new providers to existing accounts. | ✅ Designed in via oauth_accounts table structure |

##### Migration Path for Future Providers

Adding another provider later (e.g., Facebook or Discord if demand exists) requires:
1. Update `CHECK` constraint → add new value
2. Add provider-specific configuration (client ID, secret, auth/token URLs) — already parameterized in our flow
3. Handle any provider-specific quirks (scopes, profile data mapping)

The architecture is built to scale — the only real change is adding a string to the enum and some config values.

#### Open Questions Resolved

| # | Question | Decision | Rationale |
|---|----------|----------|-----------|
| Auth strategy | **Google + Apple + GitHub Sign-In** ✅ | Covers virtually all users across consumer and developer demographics. All three use standard OAuth 2.0 flows; Apple is required for future iOS native app, GitHub welcomes open-source contributors. Eliminates password management entirely. Three providers = maximum reach without complexity overload. |


### 3. Fork Graph Visualization & Lineage

**Core Concept:** A "Fork" creates a completely independent recipe that points back to its source. Unlike GitHub where you fork an entire repository, in Noms you are forking a **specific version of a specific recipe**. 

When User B forks Recipe A (at Version 3), they create Recipe B (starting at Version 1). Recipe B now has its own independent version history, but it maintains an immutable link back to the exact moment it was forked.

#### The Mechanics of Forking

**The "Cut" Point:**
A fork is a snapshot in time. If User A updates their original recipe later (adding new ingredients), User B's forked copy remains unchanged at Version 3 forever unless User B manually edits it. This ensures that forks are stable and don't unexpectedly break just because the parent changed.

**Data Model Refinement:**
Our `fork_relationships` table is already designed to handle this:
```sql
CREATE TABLE fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- The Recipe being created (the child)
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id), 
    
    -- The Source Recipe (the parent)
    original_recipe_id UUID NOT NULL REFERENCES recipes(id), 
    
    -- The specific snapshot that was copied
    original_version_id UUID NOT NULL REFERENCES recipe_versions(id), 
    
    forked_by UUID NOT NULL REFERENCES users(id),
    message TEXT,  -- e.g., "Making this vegan!" or "Adding my own twist"
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Crucial constraint: Prevent forking yourself (optional UX choice, but good data hygiene)
CHECK (forked_recipe_id != original_recipe_id),
```

#### UI Visualization Patterns

We need two ways to display lineage: **Contextual Attribution** (simple) and **The Genealogy Graph** (complex).

##### 1. Contextual Attribution (The Breadcrumbs)
At the top of every recipe page, users immediately see where it came from without cluttering the cooking instructions.

```html
<!-- Recipe Header -->
<div class="recipe-meta">
    <h1>Ultimate Garlic Bread</h1>
    
    <!-- If this is a forked recipe: -->
    <div class="lineage-breadcrumb">
        <span class="label">Forked from:</span>
        <a href="/recipes/original-id" class="recipe-link">Classic Garlic Bread by @ChefAnna</a>
        <span class="timestamp">2 years ago</span>
    </div>

    <!-- If this is the root recipe with known children: -->
    <div class="fork-count-badge">
        🍴 Forked 42 times
    </div>
</div>
```

##### 2. The Genealogy Graph (Interactive DAG)
For power users and "Recipe Detectives" who want to see the full evolution of a dish, we provide an interactive visualization. 

**Visual Language:**
- **Nodes:** Represent individual recipes (or specific versions).
- **Edges:** Arrows pointing from Child → Parent (showing origin).
- **Layout:** A horizontal tree layout works best. The current recipe is on the right; its ancestry stretches to the left.

**Interaction Design:**
1.  **"View Lineage" Button:** Toggles a full-screen modal or drawer.
2.  **Focus Mode:** Clicking any node in the graph centers the view and highlights that specific branch of the family tree, dimming unrelated branches.
3.  **Version Drill-down:** Hovering over an edge shows *which version* was forked (e.g., "Forked at v3").

**Technical Implementation (Dioxus + WebAssembly):**
We will render this as an SVG canvas using Dioxus components.
- `Node` component: Renders the user avatar, recipe title, and date.
- `Edge` component: Renders a bezier curve connecting nodes.
- **Data Fetching:** We don't send the *entire* graph to the client at once. We fetch the immediate parent/children first, then allow users to "expand" older generations (lazy loading) via API calls.

#### Backend Querying: The Ancestry Chain

To render the lineage breadcrumb or the initial graph load, we need an efficient query that walks backwards up the tree. PostgreSQL Recursive CTEs are perfect for this.

**Query: Get Full Lineage of a Recipe**
```sql
WITH RECURSIVE recipe_lineage AS (
    -- Anchor member: Start with the target recipe
    SELECT 
        r.id as recipe_id,
        r.title,
        u.username as owner,
        fr.original_recipe_id as parent_recipe_id,
        0 as depth,
        ARRAY[r.id] as path_ids  -- Prevents infinite loops if data is corrupt
    
    FROM recipes r
    JOIN users u ON r.owner_id = u.id
    LEFT JOIN fork_relationships fr ON r.id = fr.forked_recipe_id
    WHERE r.id = :target_recipe_id

    UNION ALL

    -- Recursive member: Walk up to the parent
    SELECT 
        p.title,
        pu.username as owner,
        pf.original_recipe_id as parent_recipe_id,
        rl.depth + 1 as depth,
        rl.path_ids || p.id
    
    FROM recipe_lineage rl
    JOIN fork_relationships fr ON rl.parent_recipe_id = fr.forked_recipe_id
    JOIN recipes p ON fr.original_recipe_id = p.id
    JOIN users pu ON p.owner_id = pu.id
)
SELECT * FROM recipe_lineage ORDER BY depth DESC;
```

**Query: Get Descendants (Who forked this?)**
To show the "Forked 42 times" badge, we can either run a recursive query or maintain a denormalized counter on the `recipes` table that updates via a database trigger whenever a new row is inserted into `fork_relationships`. Initially, a simple count query is fine; later, triggers will optimize this.

#### Edge Cases & Constraints

| Scenario | Handling Strategy |
| :--- | :--- |
| **Forking a Private Recipe** | Users can only fork recipes that are publicly visible to them (either Public or shared with them). We cannot allow forking private data without permission. |
| **Deep Fork Chains** | A recipe forked 10 times deep (`A -> B -> C ... -> J`). The UI should gracefully handle this by truncating the breadcrumb: "Forked from [Recipe H] (via 8 others)". |
| **Circular References** | Technically impossible with our `CHECK` constraint, but data corruption could cause it. Our SQL query uses a `path_ids` array to detect cycles and abort recursion if a node repeats. |

#### Advanced Architectural Considerations

##### 1. Single-Parent vs. Multi-Parent Flexibility
We enforce a **single-parent model** (a recipe is forked from exactly one source), but our database schema is intentionally designed to support multi-parenting later without migrations.
The `fork_relationships` table does not enforce a strict 1:1 relationship on `forked_recipe_id`. If we later decide to allow "Git-style merges" (e.g., combining Recipe A's sauce with Recipe B's crust), we simply allow multiple rows in `fork_relationships` pointing to different `original_recipe_id`s for the same child recipe. The current structure naturally scales into a true Directed Acyclic Graph (DAG) when that use case emerges.

##### 2. Deep Lineage Performance: Materialized Paths
Recursive CTEs are elegant but can degrade on extremely deep or wide viral chains (50+ forks). To prepare for this without premature complexity, we will implement **Materialized Paths** directly in the `recipes` table. 
Every recipe stores an array of its own complete ancestry history: `lineage_path UUID[]`.
- When Recipe B is forked from A, B's path becomes `[A.id, B.id]`.
- If C forks B, C's path automatically becomes `[A.id, B.id, C.id]`.

This transforms complex recursive queries into instant, indexed lookups. Finding all descendants of a recipe becomes a simple `WHERE :recipe_id = ANY(lineage_path)`, which runs in milliseconds regardless of tree depth. We will use a PostgreSQL trigger to automatically maintain this array on every new fork, keeping the application code clean.

##### 3. Visualization Layout Algorithm
Rendering a hierarchy in SVG can easily devolve into a tangled "spaghetti graph" if nodes are placed naively. To ensure an intuitive, user-friendly experience, we will implement a **Reingold-Tilford Tree Layout** algorithm directly in Rust for the Dioxus frontend.

Because we're using a single-parent model, the data structure is technically a tree (or forest). The Reingold-Tilford algorithm recursively calculates optimal X/Y coordinates based on node depth and sibling count, guaranteeing zero overlap and balanced spacing.
- **Implementation:** We will write a lightweight layout function in Rust that takes the fetched lineage JSON and returns an array of `{ x, y }` coordinates for each recipe node.
- **Rendering:** Dioxus will map these coordinates to SVG `<circle>` (nodes) and `<path>` (edges with bezier curves). 
- **Interactivity:** We'll implement viewport virtualization—if a recipe has been forked 100 times, we only render the nodes currently visible on screen, keeping the WASM bundle performant even for massive family trees.


### 4. Image Upload & Storage (Cloudflare R2)

**Philosophy:** Recipe photos are central to engagement, but they are also expensive and slow if handled poorly. Our strategy prioritizes **fast delivery via CDN**, **zero egress costs**, and **secure direct uploads from the browser** to avoid overloading our Axum backend with large binary data streams. We will focus strictly on **user-uploaded images only** (no hotlinking external blogs) to eliminate copyright complications and dead links.

#### Architecture: Presigned URLs & Direct-to-Cloud Uploads

Instead of uploading an image `Browser -> Axum Server -> R2` (which doubles bandwidth usage and ties up server memory), we use **Presigned URLs**:
1.  **Request:** User selects a photo in the browser. Dioxus sends a lightweight JSON request to Axum: "I want to upload a 3MB food photo."
2.  **Authorization:** Axum checks if the user is authenticated and has permission, then asks Cloudflare R2 for a temporary, one-time-use URL (valid for ~5 minutes).
3.  **Upload:** The browser uploads the file *directly* to Cloudflare's edge network using that secure URL (`PUT` request).
4.  **Completion:** Once uploaded, the browser notifies Axum: "It's done! Here's the public key." Axum then saves this URL into the `recipe_versions` JSONB column in PostgreSQL.

**Why this matters for Rust/Axum:** This keeps our backend extremely lightweight and performant. Axum handles logic; Cloudflare handles heavy I/O.

#### Bucket Structure & Naming Strategy

We will use a single R2 bucket (`noms-media`) with a structured internal path hierarchy to keep things organized without needing thousands of buckets.

```text
noms-media/
├── recipes/
│   ├── {recipe_id}/
│   │   ├── hero_{timestamp}_{random}.jpg      # The main cover photo
│   │   └── step-{number}_{timestamp}.webp     # Step-by-step photos
├── avatars/
│   └── {user_id}_avatar.jpg                   # Profile pictures
```

**File Naming:** We append a timestamp and random string (e.g., `hero_1715429000_a1b2c3.jpg`) to ensure **content-hash immutability**. If a user updates their hero image, the old file remains in R2 until a lifecycle policy deletes it, but the new URL is completely unique. This prevents browser caching issues where users see an old photo because their cache hasn't refreshed.

#### Cloudflare R2 Integration (Rust)

We will use the standard `aws-sdk-s3` crate, pointed at the Cloudflare R2 endpoint.

```rust
use aws_sdk_s3::{Client, Config};
use aws_smithy_types::body::SdkBody;

// Configuration loaded from Railway environment variables
async fn create_r2_client() -> Client {
    let config = Config::builder()
        .region("auto") // R2 is global, but S3 SDK requires a region string
        .credentials_provider(aws_credential_types::Credentials::new(
            std::env::var("R2_ACCESS_KEY_ID").unwrap(),
            std::env::var("R2_SECRET_ACCESS_KEY").unwrap(),
            None, None, "r2"
        ))
        .endpoint_url(format!(
            "https://{}.r2.cloudflarestorage.com", 
            std::env::var("R2_ACCOUNT_ID").unwrap()
        ))
        .force_path_style(true) // Required for R2 compatibility
        .build();

    Client::from_conf(config)
}
```

**Generating a Presigned URL in Axum:**
```rust
use aws_sigv4::http_request::{SignableBody, SigningSettings};
// ... inside an Axum route handler
let presigner = client.put_object()
    .bucket("noms-media")
    .key(format!("recipes/{}/{}", recipe_id, filename))
    .content_type(content_type) // e.g., "image/jpeg"
    .presigned(
        aws_sdk_s3::presigning::Presinging::new(&tokio::time::sleep(Duration::from_secs(300)))
            .settings(SigningSettings::default())
    )
    .send()
    .await?;

// Return the presigned URL and fields to the frontend as JSON
json!({ "url": presigner.url, "fields": presigner.fields })
```

#### Image Optimization: R2 On-the-Fly Resizing

Instead of writing complex browser-side WASM compression or running heavy server-side image processing jobs on our Rust backend, we will leverage **R2's native on-the-fly image resizing**. 

When a user uploads a massive 5MB hero photo, it is stored as-is. When the frontend requests the image, we append URL parameters like `?width=300&format=avif`. Cloudflare's edge network automatically resizes and converts the image before delivering it to the browser.

**Cost & Compute Analysis:**
*   **Compute Expenses:** R2 transforms are billed per request, but they are incredibly cheap (fractions of a cent for thousands of operations). 
*   **Why it wins:** It offloads 100% of the CPU-heavy image processing away from our Railway servers. More importantly, it drastically reduces bandwidth costs—serving a 50KB AVIF thumbnail to a mobile user instead of a 3MB JPG saves massive amounts of egress data (even with R2's zero-egress pricing tier, it improves load times and CDN efficiency).

#### Handling Recipe Imports

We will **not** automatically scrape or hotlink images from external blog URLs.
*   **Rationale:** External domains frequently change, take down old posts, or implement aggressive anti-hotlinking protections that break our UI. Furthermore, handling copyright for scraped images is legally murky.
*   **UX Flow:** When a user imports a recipe via URL and parses the text/ingredients successfully, we will give them a clean placeholder prompt: *"No photo yet? Upload your own version of this dish!"* This encourages original content and keeps our R2 bucket strictly populated with user-owned assets.

#### CDN Caching & Privacy Strategy

Since recipes can be toggled between **Public** and **Private**, our caching strategy must adapt to prevent data leaks:

| Recipe Visibility | Cache-Control Header | CDN Behavior |
| :--- | :--- | :--- |
| **Public** | `public, max-age=86400` | Aggressively cached by Cloudflare's global edge network. The first user loads the full image; everyone else in that region gets it instantly from RAM. |
| **Private** | `private, no-store` | Bypasses the CDN entirely. Every request goes straight to the origin (R2) and is served using a temporary signed URL that expires quickly. This ensures User A cannot accidentally load User B's secret family recipe via a shared edge cache node. |

#### Security Considerations

| Concern | Mitigation Strategy |
| :--- | :--- |
| **Malicious Uploads** (e.g., uploading an executable named `image.jpg`) | R2 stores data as blobs. The browser handles the rendering. We enforce strict file size limits (max 10MB) and MIME-type validation on the Axum side *before* generating the presigned URL. |
| **Unauthorized Access** | Presigned URLs are single-use and expire in 5 minutes. A user cannot use a URL meant for `recipe_1` to overwrite `recipe_2`. |

#### Blurhash: Perceived Performance Placeholders

**Problem:** When browsing the recipe feed, images load asynchronously. Without placeholders, users see white space that jumps around as images arrive — poor Cumulative Layout Shift (CLS), janky feel, and the app looks slow even when it isn't.

**Solution:** Generate a **blurhash** for every uploaded image during upload confirmation. The blurhash is a ~27-character ASCII string that encodes the dominant colors of an image as a tiny gradient. On the client side, we decode this into a CSS `background-image` and render it instantly while the full-resolution transformed image loads via Cloudflare CDN. When the real image arrives, we fade/slide it in over the gradient for a smooth transition.

**Why server-side (Axum) instead of async Worker:**
- The blurhash generation (~100ms) is absorbed into the existing upload confirmation step — user has already waited seconds for the R2 PUT to complete, so an extra 100ms is imperceptible
- Keeps the pipeline simple: one synchronous path (upload → confirm → generate hash → save) rather than coordinating async Worker events with SSE/WebSocket updates
- No additional infrastructure cost or complexity

**Upload flow with blurhash:**
```
Browser PUTs image directly to R2 via presigned URL
    │
    ▼
Browser POSTs completion metadata to Axum (/api/images/uploaded)
    │
    ├── 1. Axum fetches image from R2 (internal S3 call, fast on Cloudflare network)
    ├── 2. Generates blurhash using `fast_blurhash` crate (pure Rust, SIMD-optimized)
    ├── 3. Stores blurhash + dimensions in DB alongside the R2 key
    └── 4. Returns image metadata JSON to frontend including blurhash string
```

**Rust implementation:**
```rust
use fast_blurhash::{compute_dct_iter, BlurHash};
use image::GenericImageView;

async fn generate_blurhash(r2_client: &S3Client, key: &str) -> Result<String> {
    // Fetch from R2 via internal endpoint (fast, same network)
    let response = r2_client.get_object().bucket("noms-media").key(key).send().await?;
    let bytes = response.body.collect().await?.into_bytes();

    // Decode to pixels and generate blurhash (4×3 components = good quality/speed tradeoff)
    let img = image::load_from_memory(&bytes)?;
    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();

    let hash: BlurHash = compute_dct_iter(rgba.iter().copied(), width as u32, height as u32, 4, 3);
    Ok(hash.into_blurhash()) // e.g., "LlMF%n00%#MwS|WCWEM{R*bbWBbH"
}
```

**Client-side rendering:** The blurhash string is embedded in the HTML. A small JS decoder (or WASM module) converts it to a CSS gradient:
```css
.recipe-card-image {
    /* Instant gradient placeholder from blurhash */
    background-color: #6a5c4f;
    background-image: linear-gradient(to right, #8b7355 0%, #5a4e3d 100%);
}
```

**Cost:** Negligible. `fast_blurhash` is pure Rust with SIMD optimizations — encoding a typical food photo (3000×2000) takes ~50–100ms on Railway's CPU. The one-time R2 fetch uses internal Cloudflare network bandwidth, not egress.

#### Image Moderation Strategy

**Decision: Manual reporting first, automated screening later.** Community platforms can't launch with perfect moderation, but they also shouldn't over-engineer it before they have a problem. We ship a lightweight report/flag system from day one and layer on AI automation when upload volume justifies the cost.

**Phase 1–2 (Launch through early growth):**
- Every public recipe displays a **"Report"** button accessible via the ⋮ menu
- Reports are categorized: `spam`, `inappropriate_content`, `copyright_infringement`, `other`
- Reported images queue in an admin moderation panel (even if that's just one person clicking through)
- Actions available: warn user, remove image, remove recipe, suspend account

**Phase 3+ (When automated screening becomes cost-effective):**

| Provider | Cost Estimate | What It Detects | Notes |
|----------|--------------|-----------------|-------|
| **Sightengine** API | ~$1/1,000 images | NSFW, gore, offensive content with confidence scores | Purpose-built for moderation, REST API, easy to integrate into upload pipeline |
| Self-hosted model (Replicate-style) | GPU infra cost | Same + customizable thresholds | Only worth it at >50k uploads/month — not relevant early on |

At 100 images/day (very generous for early stage), Sightengine would cost ~$3/month. We add this when manual moderation becomes a bottleneck, not before.

**ResNet-50 auto-tagging runs separately from moderation** and is covered below.

#### AI Auto-Tagging via Cloudflare Workers AI

**Idea:** When a user uploads a recipe photo, automatically detect what's in it using `@cf/microsoft/resnet-50` and suggest relevant tags. User uploads a picture of lasagna → ResNet detects "lasagna" (94% confidence) → we auto-suggest `#Lasagna`, `#Italian`, `#Pasta`.

**How it fits into the upload pipeline:**
```
After blurhash generation in Axum:
    │
    ├── 1. Downscale image to 224×224 (ResNet input size) — cheap, local operation
    ├── 2. POST resized bytes to Cloudflare Workers AI endpoint
    ├── 3. Parse top-5 classifications with confidence scores > threshold (e.g., >0.6)
    ├── 4. Map ImageNet class names → our tag vocabulary:
    │      "pizza"       → #Pizza, #Italian
    │      "salad bowl"  → #Salad, #Healthy
    │      "steak"       → #Steak, #Beef, #Grilling
    ├── 5. Store suggested tags as `pending_tags` on the image record
    └── 6. Frontend shows them to user during recipe editing: "Auto-detected tags (click to add/remove)"
```

**Cost analysis:** ResNet-50 costs **$2.51 per million images**, or $0.00000251 per image. At 1,000 uploads/day that's ~$0.91/month. Essentially free at any realistic early-stage scale and well within the Workers AI free allocation of 10,000 neurons/day (~4,000 ResNet calls/day for free).

**Caveats & limitations:**
- ImageNet has 1,000 classes — many are food-relevant (pizza, cake, steak, salad) but not all. Non-food photos will return irrelevant tags (e.g., "church", "motorcycle") which we silently discard via a confidence threshold + food-category whitelist
- Auto-suggested tags are **never applied automatically** — they're always presented to the user for review. This prevents embarrassing misclassifications and keeps users in control of their content's discoverability
- We can expand the tag vocabulary over time by maintaining a mapping table: `imagenet_class_id → noms_tags[]`

## Community & Discovery Engine

**Philosophy:** The "Community Feed" should feel magical—like it knows what you like before you ask for it. We will combine explicit social signals (who you follow) with implicit behavioral data (what you view/fork) to power a robust discovery engine.

### 1. Social Interactions & Data Tracking
We need to distinguish between different types of engagement to build an accurate user profile.

| Action | Definition | Impact on Algorithm |
| :--- | :--- | :--- |
| **Follow** (User) | Explicit interest in a creator's output. | Heavy weight: Their new recipes always appear at the top of your feed. |
| **Like** (Recipe) | Public endorsement ("This is good!"). | Moderate weight: Signals taste preference for this specific dish type. |
| **Favorite** (Recipe) | Private bookmark ("I want to make this later"). | High weight: Strong indicator of cooking intent and personal preference. |
| **Fork** (Recipe) | Active adaptation ("This is a base I like"). | Highest weight: Shows deep engagement with the recipe's core concept. |

### 2. Implicit Interest Modeling
Instead of asking users to fill out a complex "profile" of what they eat, we will build an **Interest Vector** based on their activity history.

- When a user interacts with recipes containing specific tags (e.g., `#Italian`, `#Vegan`, `#Quick`), we increment counters in a lightweight profile table.
- Over time, if User A likes/forks 10 "Spicy" dishes and only 2 "Sweet" dishes, their implicit preference score for Spicy food increases.
- **Privacy First:** This data is used *only* to sort their personal feed and search results. We don't display a public "Interest Graph."

### 3. Search Architecture: Text + Semantic
We will implement a hybrid search strategy using PostgreSQL's built-in capabilities plus the `pgvector` extension.

#### A. Keyword Search (The Foundation: pg_search)
Instead of standard Postgres full-text search, we will use the **`pg_search`** extension. It provides "Elasticsearch-like" capabilities directly inside PostgreSQL without adding external infrastructure.

**Why `pg_search` is essential for recipes:**
*   **Fuzzy Matching (Typos):** If a user types "chiken parm", standard Postgres returns nothing. `pg_search` understands the intent and returns "Chicken Parmesan" automatically.
*   **Faceting & Filtering:** It natively supports complex filters without heavy SQL joins. E.g., *"Show me recipes where Difficulty = 'Easy' AND Cuisine contains 'Italian'"*.
*   **Stemming:** Understands that "baking", "baked", and "bakes" are the same concept.

**Implementation Note:** Since we are hosting on Railway, standard Postgres images don't always include custom extensions out-of-the-box. **`pg_trgm` ships with vanilla PostgreSQL** and is available everywhere — no special image needed. For `pgvector` and `pg_search`, we will likely need a community-maintained Postgres image (like those from `Supabase`) that includes both enabled via Docker configuration.

```sql
-- Example: Fuzzy search for "recpie" using pg_search
SELECT * FROM recipe_versions 
WHERE @@@('recipe_name', 'recpie') -- Handles typo tolerance automatically;
```

#### B. Semantic Search (The "Magic")
This allows users to search by concept rather than keywords. A search for *"something spicy and fast"* should return recipes tagged `#Spicy` with `<30min` cook times, even if the text doesn't contain those exact words.

**Implementation: pgvector Embeddings:**
1.  **Vectorizing Recipes:** When a recipe is saved (or updated), we send its title + ingredients to an embedding model API (like OpenAI's `text-embedding-3-small`). This returns a dense vector of numbers (e.g., 1536 dimensions) representing the "meaning" of the dish.
2.  **Storage:** We store this array in a `vector` column in Postgres using the `pgvector` extension.
3.  **Querying:** When a user types a query, we convert their text into a vector and find the nearest neighbors using Cosine Distance.

```sql
-- Find recipes semantically similar to "Spicy weeknight dinner"
SELECT id, title, embedding <-> :query_vector AS distance 
FROM recipe_versions 
ORDER BY distance ASC 
LIMIT 10;
```

#### C. Search Autocomplete (The "Type-Ahead" Experience: pg_trgm)

As users type into the search bar, we want instant suggestions — recipe titles, ingredient names, and tags that match what they're typing in real time. This is fundamentally different from full search (which runs after pressing Enter). Autocomplete needs **sub-50ms responses**, partial/infix matching ("garlic" matches "Roasted Garlic Bread"), and must handle typos gracefully.

**Why `pg_trgm` is the right tool:**
The PostgreSQL Trigram module (`pg_trgm`) decomposes every indexed string into overlapping 3-character sequences (trigrams). The string `"chicken"` becomes `{ ch, hi, ic, ck, ke, en, n_ }`. When a user types "ick", Postgres looks for rows containing those trigrams — instantly matching "ch**ick**en" even though the substring appears mid-word.

| Capability | `pg_trgm` | Standard `LIKE` | Why It Matters |
| :--- | :--- | :--- | :--- |
| **Prefix match** (`garlic%`) | ✅ Fast (uses GIN index) | ✅ Fast (B-tree) | Both work, trigram is more versatile |
| **Infix match** (`%alic%`) | ✅ Fast (uses GIN index) | ❌ Full table scan | Users type "chick" expecting "Chicken Parmesan" — infix matching is essential for autocomplete UX |
| **Fuzzy similarity** (`<%`, `%>`) | ✅ Built-in `similarity()` function | ❌ Not supported | Typing "recpie" still surfaces "Recipe Name" because the trigram overlap score is high enough |
| **Ranked results** | ✅ `similarity()` returns 0.0–1.0 score | ❌ Binary match/no-match | We can sort by relevance, showing closest matches first |

**Implementation:**
```sql
-- Enable the extension (one-time setup)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Add a GIN trigram index on recipe titles for sub-50ms infix lookups
CREATE INDEX idx_recipes_title_trgm ON recipes USING GIN (title gin_trgm_ops);

-- Optional: Also index ingredient names and tags for broader autocomplete coverage
CREATE INDEX idx_recipe_versions_ingredients_trgm 
    ON recipe_versions USING GIN ((ingredients::text) gin_trgm_ops);

-- Autocomplete query: User types "garlic br"
SELECT title, id
FROM recipes
WHERE title % 'garlic br'  -- The "%" operator = similarity threshold (default >0.3)
ORDER BY similarity(title, 'garlic br') DESC
LIMIT 8;
```

**Query characteristics:**
- **Latency:** With a GIN trigram index on ~100K recipes, infix searches complete in **<5ms** because Postgres only scans the inverted trigram index, not the full table.
- **Threshold tuning:** The `%` operator uses a default similarity threshold of 0.3 (roughly "at least one common trigram"). We can tune this via `SET pg_trgm.similarity_threshold = 0.4;` if results are too noisy early in typing, or lower it for more permissive matching on short queries (< 4 characters).
- **Minimum query length:** On the Axum side, we'll debounce the frontend input (300ms) and only fire the API request after ≥2 characters have been typed. Single-character queries return nothing — avoiding wasteful database hits and noisy results.

**API Endpoint Design:**
```
GET /api/search/autocomplete?q=garlic&limit=8
→ [{ "type": "recipe", "id": "...", "title": "Garlic Butter Pasta" }, 
    { "type": "ingredient", "text": "garlic powder" },
    ...]
```

**Relationship to the broader search architecture:**
`pg_trgm`, `pg_search`, and `pgvector` each serve a distinct phase of the user's search journey:

| Phase | Extension | Triggered When... | Example |
| :--- | :--- | :--- | User types "gar" → dropdown shows "Garlic Bread", "Garam Masala Chicken" |
| **Autocomplete** (type-ahead) | `pg_trgm` | ...user is actively typing, ≥2 chars, debounced 300ms | — |
| **Keyword Search** (precision) | `pg_search` | User presses Enter or selects a suggestion | "chiken parm" → fuzzy matches "Chicken Parmesan" with faceted filters |
| **Semantic Search** (intent) | `pgvector` | User types natural language queries ("something quick and spicy for weeknight") | Cosine similarity on embeddings returns conceptually relevant recipes |

All three coexist in the same database, zero external search infrastructure required. The autocomplete layer (`pg_trgm`) is the fastest and lightest — it's purely string-based with no embedding API calls or heavy parsing involved.
For the initial feed implementation, we will use a **Weighted Scoring System** rather than a complex ML model. Every time a user opens their "Discover" tab, we calculate a score for candidate recipes:

`Score = (Social_Signal * 2) + (Interest_Match * 3) - (Recency_Decay)`

*   **Social Signal:** Is this from someone I follow? (+10 points).
*   **Interest Match:** Does the recipe's tags match my top 3 implicit interests? (+5 points per tag).
*   **Recency Decay:** Recipes posted yesterday score higher than recipes posted a month ago.

This ensures the feed is always fresh, personally relevant, and socially connected.

### 5. Offline Usability & Sync Strategy

**Problem:** Users cook in kitchens where WiFi is unreliable or absent — behind thick walls, on balconies, at farmers markets jotting down inspiration, on phones with limited data plans. A recipe app that stops working without signal breaks at the exact moment users need it most. Offline access must feel seamless, not like a degraded fallback mode.

**Philosophy:** Full offline-read and offline-write for all personal content. Users can browse their recipes, create new ones, edit existing versions, and favorite external recipes — entirely disconnected from the server. Changes sync automatically when connectivity returns with no manual intervention required. The community feed and discovery features require a network connection (acceptable limitation), but everything personal works everywhere.

#### Architecture Overview: Three-Layer Offline System

| Layer | Technology | Responsibility |
|---|---|---|
| **App Shell** | Service Worker + Cache API | Caches WASM binary, CSS bundles, JS assets for instant cold starts without network |
| **Recipe Data** | SQLite (via `op-sqlite` WASM) | Full relational database running in-browser — stores personal recipes, versions, favorites, authors. Supports identical SQL queries as the online PostgreSQL backend |
| **Images** | Service Worker Cache API (stale-while-revalidate) | Recipe hero photos and step images cached transparently on first view; served from cache on subsequent visits whether online or offline |

#### Why SQLite Over IndexedDB

IndexedDB is a key-value store with no query language. To find "all my vegan pasta recipes" in IndexedDB, you'd load everything into memory and filter in Rust — O(n) every time. SQLite gives us full SQL: `SELECT * FROM recipes JOIN recipe_tags ON ... WHERE tag IN ('vegan', 'pasta')` runs in milliseconds against a local index.

More importantly, **SQLite enables a shared query layer** between the Axum backend (PostgreSQL) and Dioxus frontend (SQLite). The same SQL queries run against either database engine — we swap the connection, not the code. This eliminates maintaining two entirely different data access patterns for online vs offline mode.

#### Platform Strategy: WASM SQLite Works Everywhere We Ship Today

Dioxus targets four rendering backends. Here's how SQLite fits across each:

| Platform | Dioxus Renderer | Execution Environment | Database Layer |
|---|---|---|---|
| **Web** | `dioxus-web` (WASM) | Browser JS engine + WASM runtime | `op-sqlite` WASM — native home, zero friction |
| **Desktop** | `dioxus-desktop` (webview via Wry/Tao) | Chromium webview embedding a WASM app | `op-sqlite` WASM — same binary as web. The host Rust process provides window chrome; our app + database run in the embedded webview's WASM sandbox |
| **Mobile** | `dioxus-mobile` (webview) | iOS WKWebView / Android WebView with WASM support | `op-sqlite` WASM — same binary as web. Identical behavior across platforms |
| **Native GPU** | `dioxus-native` (Blitz renderer) | Pure Rust binary, no browser/WASM at all | `rusqlite` native — standard SQLite C library binding. No WASM involved |

**Key realization:** For desktop and mobile using Dioxus's default webview renderers, our app still executes as WASM inside a browser sandbox. The host process just provides window chrome and platform API bridges. There is no advantage to native SQLite here — we can't reach it from inside the webview without building an IPC bridge that adds complexity for zero capability gain. So `op-sqlite` WASM covers three of four targets with one compiled binary.

The native GPU renderer (`dioxus-native`) would use standard `rusqlite`, but our trait-based abstraction (see below) makes that a surgical swap behind a feature flag, not a rewrite.

#### Crate Choice: `op-sqlite`

We will use [`op-sqlite`](https://github.com/opendb/op-sqlite) — the OpenJS Foundation reference implementation for SQLite compiled to WASM. It provides an API nearly identical to `rusqlite`, with async methods that yield to the browser event loop during long-running queries (critical — we cannot block the JS main thread).

**Why `op-sqlite` over alternatives:**
- **OpenJS Foundation backing:** Actively standardized as part of the "WASM SQL" initiative. Projects like Bun, Cloudflare Workers, and Deno invest in this ecosystem. We benefit from performance improvements and bug fixes driven by larger organizations.
- **Broader WASM target coverage:** Supports `wasm32-unknown-unknown` (browser) + `wasm32-wasip1` (Node.js/Bun/Deno). If we later need a server-side WASM deployment path, the same crate works.
- **FTS5 support:** Full-text search virtual table is available and actively tested as part of spec compliance — gives us offline recipe search for free.
- **Bundle size:** ~1MB WASM — acceptable cost given it replaces what would otherwise be a custom IndexedDB query layer of comparable complexity, while providing full SQL capability.

**Browser requirement:** `op-sqlite` uses SharedArrayBuffer for performance, which requires `cross-origin-isolation` response headers (`Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp`). Railway serves these headers trivially via environment configuration. A single-threaded fallback exists if headers cannot be set (e.g., embedded contexts with restricted header control).

#### Shared Query Layer — One Codebase, Two Databases

The project includes a shared Rust crate (`query-layer`) that compiles to both native (backend) and WASM (frontend). All SQL queries live here. Both PostgreSQL (via `sqlx`/`deadpool-postgres`) and SQLite (via `op-sqlite`) implement the same trait:

```rust
// query-layer/src/lib.rs — shared abstraction over database backends

#[async_trait]
pub trait DbConn {
    async fn query_one(&self, sql: &str, params: &[DynamicValue]) -> Result<Row, DbError>;
    async fn query_many(&self, sql: &str, params: &[DynamicValue]) -> Result<Vec<Row>, DbError>;
    async fn execute(&self, sql: &str, params: &[DynamicValue]) -> Result<u64, DbError>;
    async fn transaction<F, R>(&self, f: F) -> Result<R, DbError>
    where F: FnOnce(&dyn DbConn) -> Result<R, DbError>;
}

// Backend implementation — delegates to sqlx PgPool
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl DbConn for PostgresDb { /* ... */ }

// Frontend implementation — delegates to op-sqlite Connection (WASM)
#[cfg(target_arch = "wasm32")]
#[async_trait]
impl DbConn for SqliteDb { /* ... */ }

// Native renderer fallback — standard rusqlite (no WASM)
#[cfg(feature = "native-renderer")]
#[async_trait]
impl DbConn for NativeSqliteDb { /* wraps blocking calls in spawn_blocking */ }
```

A query function like `list_recipes_by_owner(db: &dyn DbConn, owner_id: Uuid)` works identically whether called from Axum (PostgreSQL) or Dioxus (SQLite). When we write a new feature on the backend, its offline equivalent works automatically — no second implementation needed.

**Conditional compilation picks the right database per target:**
- `wasm32-unknown-unknown` → compiles `op-sqlite` impl only
- `x86_64-unknown-linux-gnu` → compiles `sqlx` PostgreSQL impl only
- `aarch64-apple-ios` with `native-renderer` feature → compiles `rusqlite` impl only

The wrong implementation literally does not compile into the binary for that target. Zero runtime branching, zero dead code in production builds.

##### SQL Dialect Compatibility — Where They Share and Where They Don't

PostgreSQL and SQLite share a common ancestor (ANSI SQL) but have diverged significantly in extension areas. The honest breakdown: ~75% of our queries are identical text across both databases; the remaining ~25% branch on `#[cfg]` into small dialect-specific implementations. This is still dramatically better than maintaining two separate data access layers entirely.

| Query Pattern | PostgreSQL | SQLite | Shared? | Frequency in Noms |
|---|---|---|---|---|
| **Basic CRUD** (`SELECT`, `INSERT`, `UPDATE`, `DELETE` with parameterized values) | Identical ANSI SQL | Identical ANSI SQL | ✅ 100% — one string, no branching | ~80% of all queries (list recipes, get by ID, create version, delete recipe, etc.) |
| **Joins, subqueries, `GROUP BY`, `ORDER BY`, `LIMIT`/`OFFSET`** | Standard SQL-92+ | Standard SQL-92+ | ✅ Nearly identical. Minor edge cases on NULL sorting — avoidable by convention | High — core data retrieval |
| **Window functions** (`ROW_NUMBER()`, `RANK()`) | Full support | Full support (3.25+) | ✅ Identical syntax | Low — feed ranking, pagination helpers |
| **Recursive CTEs** (`WITH RECURSIVE`) | Since 8.4 | Since 3.8.3 | ✅ Identical syntax and semantics | Medium — fork lineage traversal, collection hierarchy |
| **Transactions** (`BEGIN`/`COMMIT`/`ROLLBACK`, savepoints) | Standard | Standard | ✅ Identical | Every write operation |
| **ISO-8601 timestamp comparison** | `TIMESTAMPTZ` native type | TEXT with lexicographic sort (works for ISO format) | ✅ Queries identical if we compare as strings (`WHERE updated_at > '2024-...'`) | High — sync filtering, sorting by date |
| **UUID comparison** | Native `uuid` type | TEXT only | ✅ Identical if client generates UUIDs (our plan) — stored and compared as strings everywhere | Every primary key lookup |
| **JSON querying inside SQL** (`->`, `@>`, `jsonb_build_object()`) | Rich operator set (`->`, `->>`, `@>`, containment) | Function-based only (`json_extract()`, `json_each()`) — different names, no operators | ❌ Must branch. Different API surface entirely | Low — we mostly read JSONB columns as opaque blobs and parse in Rust. Rare cases: ingredient name search inside JSON arrays |
| **Native array operations** (`ANY`, `@>`, `<@`) | First-class arrays with rich containment/prefix operators | No native array type (would encode as JSON text) | ❌ Branch required for materialized path queries like `WHERE :id = ANY(lineage_path)` | Low — collections hierarchy lookup, fork lineage ancestry check. Two query patterns total |
| **Full-text search** | `@@` with `tsvector`/`to_tsquery()` or `pg_search` syntax | FTS5 virtual table with `MATCH` operator on a separate content table | ❌ Unavoidable — fundamentally different indexing engines, entirely different query syntax | One query pattern (search), handled behind a function abstraction |

**The math:** Out of ~30-40 unique query functions across the app (CRUD + search + collections + sync), only 3-5 need dialect-specific implementations. The rest are one SQL string that compiles for both targets with no `#[cfg]` touching them.

##### How We Handle Branching — Isolated, Not Scattered

Dialect-specific queries are isolated into their own files/functions. The branching is surgical — five small functions with two implementations each behind compile-time flags — not a sprawling conditional mess:

```rust
// query-layer/src/search.rs — ONLY file that branches for FTS dialect differences

#[cfg(target_arch = "wasm32")] // SQLite / offline — uses FTS5 virtual table
pub async fn search_recipes(db: &dyn DbConn, query: &str) -> Result<Vec<Recipe>, DbError> {
    db.query_many(
        r#"SELECT r.* FROM recipes r
           JOIN recipes_fts f ON f.rowid = r.rowid
           WHERE f MATCH ?
           ORDER BY rank LIMIT 20"#,
        &[query],
    ).await
}

#[cfg(not(target_arch = "wasm32"))] // PostgreSQL / online — uses pg_trgm similarity + FTS
pub async fn search_recipes(db: &dyn DbConn, query: &str) -> Result<Vec<Recipe>, DbError> {
    db.query_many(
        r#"SELECT * FROM recipes
           WHERE title % $1 OR to_tsvector('english', title || ' ' || COALESCE(description, '')) @@ plainto_tsquery('english', $1)
           ORDER BY similarity(title, $1) DESC NULLS LAST
           LIMIT 20"#,
        &[query],
    ).await
}
```

Collections materialized path — one tiny branching helper:

```rust
// query-layer/src/collections.rs

#[cfg(target_arch = "wasm32")] // SQLite — path stored as JSON text array
pub async fn collection_descendants(db: &dyn DbConn, folder_id: &str) -> Result<Vec<Collection>, DbError> {
    db.query_many(
        r#"SELECT * FROM collections
           WHERE json_extract(path, '$') = ? OR
                 json_extract(path, '$[-1]') = ?"#,  // Simplified — full implementation uses recursive JSON traversal
        &[folder_id, folder_id],
    ).await
}

#[cfg(not(target_arch = "wasm32"))] // PostgreSQL — native UUID[] column with containment operator
pub async fn collection_descendants(db: &dyn DbConn, folder_id: &str) -> Result<Vec<Collection>, DbError> {
    db.query_many(
        r#"SELECT * FROM collections WHERE path @> ARRAY[$1]"#,
        &[folder_id],
    ).await
}
```

Every other query function — `list_recipes`, `get_recipe_by_id`, `create_recipe_version`, `delete_recipe`, `get_user_favorites`, `get_recipe_versions`, `fork_recipe` — is one SQL string, zero branching. The pattern is: **share aggressively by default; branch surgically where the databases fundamentally disagree.**

#### SQLite Local Schema — Mirrors PostgreSQL Structure

The local SQLite database replicates a subset of our PostgreSQL schema — specifically the tables needed for personal offline access:

```sql
-- Local SQLite mirrors these PostgreSQL tables (identical column structure)
CREATE TABLE users (
    id TEXT PRIMARY KEY,           -- UUID as text
    username TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    sync_status TEXT DEFAULT 'synced',  -- 'synced' | 'stale'
);

CREATE TABLE recipes (
    id TEXT PRIMARY KEY,
    owner_id TEXT NOT NULL REFERENCES users(id),
    title TEXT NOT NULL,
    description TEXT,
    is_public INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,      -- ISO-8601 timestamp string
    updated_at TEXT NOT NULL,
    sync_status TEXT DEFAULT 'synced',
);

CREATE TABLE recipe_versions (
    id TEXT PRIMARY KEY,
    recipe_id TEXT NOT NULL REFERENCES recipes(id),
    version_number INTEGER NOT NULL,
    ingredients TEXT NOT NULL,     -- JSONB stored as text
    steps TEXT NOT NULL,           -- JSONB stored as text
    created_at TEXT NOT NULL,
);

CREATE TABLE recipe_tags (
    recipe_id TEXT NOT NULL REFERENCES recipes(id),
    tag_name TEXT NOT NULL,
    PRIMARY KEY (recipe_id, tag_name),
);

-- Mutation queue: offline edits waiting to sync to PostgreSQL
CREATE TABLE sync_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recipe_id TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('create', 'update', 'delete')),
    payload TEXT NOT NULL,         -- JSON snapshot of the change
    queued_at TEXT NOT NULL,
    retries INTEGER DEFAULT 0,
);

CREATE INDEX idx_sync_queue_recipe ON sync_queue(recipe_id);
CREATE INDEX idx_recipes_owner ON recipes(owner_id);
CREATE INDEX idx_versions_recipe ON recipe_versions(recipe_id, version_number DESC);

-- Full-text search index for offline recipe search
CREATE VIRTUAL TABLE recipes_fts USING fts5(
    title, description,
    content='recipes',
    content_rowid='rowid'
);

CREATE TRIGGER recipes_ai AFTER INSERT ON recipes BEGIN
    INSERT INTO recipes_fts(rowid, title, description)
    VALUES (new.rowid, new.title, new.description);
END;
```

**Key differences from PostgreSQL:**
- UUIDs stored as TEXT (SQLite has no native UUID type — trivial, just string comparison)
- JSONB columns stored as TEXT (SQLite has no JSONB type, but we only read/write them as opaque strings anyway — parsing happens in Rust application code)
- `TIMESTAMPTZ` stored as ISO-8601 text strings (`strftime()` handles comparisons)
- No foreign key cascades needed for offline use (data lifecycle is managed by the sync layer, not referential integrity rules)

These differences are cosmetic. The SQL queries that run against this schema are identical to their PostgreSQL equivalents.

#### Data Caching Policy — What Gets Stored Locally

| Category | Sync Policy | Storage Estimate |
|---|---|---|
| **Personal recipes** (all versions, all metadata) | Eager — synced on every save/edit automatically | ~5-10KB text per recipe; trivial at any realistic scale |
| **Favorited/saved external recipes** (current version only) | Eager — synced on favorite/unfavorite toggle | Scales with user activity; 500 favorites ≈ 5MB text |
| **Recently viewed recipes** (last 50 opened, current version) | Background — Dioxus task pushes to SQLite post-view. Bounded LRU: when exceeding 50, evicts oldest entries automatically | ~250KB max; self-managing |
| **Author profiles** (for cached recipes) | Included during initial seed and incremental sync for any recipe author referenced by local data | Negligible — small user record per unique author |
| **Recipe images** | Not in SQLite — handled transparently by Service Worker Cache API on first image load. Hero + step photos cache naturally as users scroll through recipes. URLs stored as text references in recipe JSON. | Browser-managed; typically hundreds of MB available |

**What we intentionally do NOT cache:**
- **Community feed / Discover tab:** Requires fresh server-side ranking. Show a friendly empty state when offline: *"Discover needs internet — check your personal recipes instead."*
- **Non-favorited external recipes:** Only favorited/saved external content persists locally. Casual browsing doesn't fill the database.
- **Comments from other users:** Ephemeral social data. Cached versions may appear stale while offline; posting new comments queues as a pending mutation.

#### Sync Lifecycle — Three Phases

**Phase 1: Initial Seed (first login or hard refresh)**

On first app load, if the local SQLite database is empty or marked stale, we fetch a compressed snapshot from Axum containing everything needed for full offline capability:

```
Client → Server:  GET /api/sync/initial-snapshot
Server → Client:  { recipes: [...], versions: [...], users: [...], synced_at: "2024-..." }
Client:           Wipe local SQLite → bulk INSERT all records in a single transaction
```

This is one request that fully seeds offline capability. After this, the user can disconnect immediately and have full read access to their personal library + favorites.

**Phase 2: Background Incremental Sync (ongoing)**

Once seeded, we switch to incremental sync using `updated_at` timestamps. A persistent Dioxus task runs silently in the background:

```
Client → Server:  GET /api/sync/incremental?since=2024-01-15T10:30:00Z
Server → Client:  { updated_recipes: [...], deleted_recipe_ids: [...] }
Client:           UPSERT into local SQLite, overwrite whatever we had locally
```

This triggers automatically when the app detects it's online. No user action required. If a recipe was edited on another device during offline time, the server version overwrites our local copy (last-write-wins — see conflict resolution below).

**Phase 3: Mutation Flush (offline edits → server)**

Every write operation performed while offline gets queued in the `sync_queue` table with its operation type and a JSON payload. When connectivity returns, the background sync worker drains it in order:

```rust
// Conceptual Dioxus task — runs persistently in background
use_effect(move || {
    spawn(async move {
        loop {
            // Wait until online AND pending mutations exist
            if !is_online() {
                sleep(Duration::from_secs(5)).await;  // Poll connectivity
                continue;
            }

            let mutations = sqlite.query_all("SELECT * FROM sync_queue ORDER BY queued_at ASC").await;
            if mutations.is_empty() {
                sleep(Duration::from_secs(10)).await;
                continue;
            }

            for mutation in mutations {
                let result = match mutation.operation.as_str() {
                    "create" => api_post("/api/recipes", &mutation.payload).await,
                    "update" => api_patch(&format!("/api/recipes/{}", mutation.recipe_id), &mutation.payload).await,
                    "delete" => api_delete(&format!("/api/recipes/{}", mutation.recipe_id)).await,
                    _ => continue,
                };

                match result {
                    Ok(response) => {
                        // Server accepted — update local SQLite with server-generated timestamps/metadata
                        apply_server_response(&sqlite, &mutation, &response).await;
                        sqlite.execute("DELETE FROM sync_queue WHERE id = ?", (&mutation.id)).await.ok();
                    }
                    Err(e) if e.status == 409 => {
                        // Conflict — pause and show user resolution dialog
                        show_conflict_dialog(mutation, server_version).await;
                        break;  // Stop processing until user resolves
                    }
                    Err(_) => {
                        // Transient error — increment retry counter, backoff
                        sqlite.execute("UPDATE sync_queue SET retries = retries + 1 WHERE id = ?", (&mutation.id)).await.ok();
                        sleep(Duration::from_secs(30)).await;
                    }
                }
            }

            if mutations.is_empty() {
                show_toast("All changes synced ✓");
            }
        }
    });
});
```

#### Offline Editing — Full Write Support

Users can create, edit, and delete recipes entirely offline. The code path is identical to the online version — it just writes to local SQLite + `sync_queue` instead of PostgreSQL:

```rust
// Works identically whether online or offline — DbConn routes to the right backend
async fn create_recipe(db: &dyn DbConn, draft: RecipeDraft) -> Result<Recipe, DbError> {
    let id = Uuid::new_v4();  // Client-generated UUID — no server round-trip needed

    db.transaction(|tx| async {
        tx.execute(
            "INSERT INTO recipes (id, owner_id, title, description, created_at, updated_at) VALUES ($1, $2, $3, $4, strftime('%Y-%m-%dT%H:%M:%SZ'), strftime('%Y-%m-%dT%H:%M:%SZ'))",
            (&id, &draft.owner_id, &draft.title, &draft.description),
        ).await?;

        tx.execute(
            "INSERT INTO recipe_versions (id, recipe_id, version_number, ingredients, steps, created_at) VALUES ($1, $2, 1, $3, $4, strftime('%Y-%m-%dT%H:%M:%SZ'))",
            (&Uuid::new_v4(), &id, &draft.ingredients_json, &draft.steps_json),
        ).await?;

        // Queue for server sync when connectivity returns
        tx.execute(
            "INSERT INTO sync_queue (recipe_id, operation, payload, queued_at) VALUES ($1, 'create', $2, strftime('%Y-%m-%dT%H:%M:%SZ'))",
            (&id, &draft.to_json()),
        ).await?;

        Ok(Recipe { id, title: draft.title.clone(), sync_status: SyncStatus::Pending })
    }).await
}
```

**Client-generated UUIDs (v4):** Every recipe, version, and mutation gets a random UUID generated locally. This means offline-created recipes have valid unique identifiers without any server round-trip. No ID collision is possible regardless of timing — two devices creating recipes simultaneously will never generate the same UUID. When the mutation syncs to the server, PostgreSQL accepts the client-provided ID (we disable `DEFAULT gen_random_uuid()` for fields that accept client IDs).

#### Conflict Resolution Strategy

Offline edits creating conflicts on the same recipe from multiple devices are rare but must be handled gracefully:

| Scenario | Resolution |
|---|---|
| **User edits recipe offline, then edits same recipe online from another device** | **Last-write-wins by `updated_at` timestamp.** The newer version wins; older changes are discarded with a toast notification: *"Your earlier edit was superseded by a change from your other device."* No manual merge required — recipes don't have the fine-grained concurrent editing patterns that documents do. |
| **User creates recipe offline, same title already exists on server** | **No conflict possible.** UUIDs are client-generated v4, so uniqueness is guaranteed regardless of timing or naming collisions. |
| **User deletes recipe offline, but another device updated it since** | Server rejects the delete with a 409 conflict. We show: *"This recipe was modified elsewhere since you deleted it. Restore your local copy or force-delete?"* User makes an explicit choice. |
| **User forks someone else's recipe offline** | Fork relationships queue as mutations. When synced, the server validates that the source recipe still exists and creates the fork relationship normally. If the source was deleted in the meantime, we show a recovery dialog. |

#### Offline Search — FTS5 Virtual Table

SQLite's `FTS5` extension gives us full-text search running entirely offline in WASM:

```sql
-- Search query runs entirely locally, sub-10ms for thousands of recipes
SELECT r.* FROM recipes r
JOIN recipes_fts f ON f.rowid = r.rowid
WHERE f MATCH 'garlic bread'
ORDER BY rank LIMIT 20;
```

The search bar works identically online and offline — same SQL query pattern, different database engine. No special "offline search" UI is needed at all. The FTS5 index stays in sync with the recipes table via SQLite triggers (INSERT/UPDATE/DELETE automatically maintain the full-text index).

#### Service Worker — App Shell & Image Caching

A minimal service worker handles static assets and images:

| Asset Type | Cache Strategy | Rationale |
|---|---|---|
| **WASM binary + CSS + JS bundles** | `cache-first` with hash-based filenames | Always serve from cache; version bump (filename change) forces fresh fetch on next hard refresh. Critical for instant cold starts — the Dioxus WASM binary is ~200-400KB and must load instantly. |
| **R2 image URLs** | `stale-while-revalidate` | Serve cached copy instantly if available; fetch fresh version in background. Images are expensive to re-download and rarely change. Works transparently — no user action needed. |
| **Axum API calls (`/api/...`)** | `network-first`, fall back to SQLite on network failure | Never serve stale API data from SW cache directly. That's the SQLite layer's job. The SW just detects offline state and lets Dioxus route queries locally. |

#### UX Indicators — Subtle, Never Alarming

The goal is to make offline feel like a feature, not a failure state:

| State | UI Signal |
|---|---|
| **Online (normal)** | No indicator at all. Invisible = good. |
| **Just went offline** | Brief toast: *"You're offline — your recipes are still available."* Auto-dismisses after 4 seconds. |
| **Persistently offline, user attempts network-only action** | Inline error on the specific button/form: *"Can't publish while offline. Changes saved locally and will sync when you reconnect."* |
| **Viewing cached recipe while offline** | Small badge in recipe header: `Offline` (muted gray, not red). Disappears automatically when online again. |
| **Background sync completing** | Subtle progress pill in bottom corner: *"Syncing 3 changes…" → "All caught up ✓"* |
| **Recipe has pending unsynced changes** | Small dot indicator next to recipe title in library view. Tooltip: *"3 unsynced changes"*. Clears after successful sync. |

#### Dioxus Integration Pattern — `use_offline()` Hook

A custom Dioxus hook provides components with a unified data source that transparently switches between network and SQLite:

```rust
pub struct OfflineContext {
    pub status: OnlineStatus,          // Online | Offline
    pub db: SqliteDb,                  // Local database connection (always available)
    pub pending_count: Signal<usize>,  // Number of unsynced mutations
}

pub fn use_offline() -> OfflineContext { /* ... */ }

// Consuming component — identical code path either way:
let offline = use_offline();
let recipes = offline.db.query_all(
    "SELECT * FROM recipes WHERE owner_id = ? ORDER BY updated_at DESC",
    [&user.id],
).await;

if offline.status == OnlineStatus::Offline {
    // Optionally show offline badge, but data is the same shape
}
```

#### Storage Budget Management

| Concern | Strategy |
|---|---|
| **Personal recipes** | Never evicted. This is the user's core data — always available locally regardless of storage pressure. |
| **Favorited external recipes** | Capped at 500 entries. When exceeded, oldest unfavorited-then-refavorited recipes are evicted first. User can explicitly "pin" favorites to prevent eviction. |
| **Recently viewed cache** | Hard cap at 50 entries with automatic LRU eviction on insert. Self-managing, no user control needed. |
| **Image cache (SW Cache API)** | Browser-managed quota (typically hundreds of MB). We never explicitly evict — the browser handles pressure automatically by removing least-recently-used cached responses. |
| **Total estimated storage** | ~5-20MB for a typical active user (text data). Images add variable overhead managed by the browser. Nowhere near browser quota limits under normal usage. |


### Key Journey 1: Import & Save a Recipe
User finds a recipe on a food blog → copies URL into Noms → app parses the page and extracts ingredients, steps, images, metadata → user reviews/edits the parsed data → saves to personal library → optionally publishes to community

### Key Journey 2: Discover via Community Feed
User opens their feed → sees new recipes from people they follow + recommended content → clicks on a recipe that catches their eye → reads through, maybe leaves a comment or like → forks it to try at home with modifications

### Key Journey 3: Fork & Customize
User finds a recipe they love but want to tweak → clicks "Fork" → gets their own copy with original attribution visible → modifies ingredients/steps → saves new version → the fork chain is visible (Original User A → Forked by User B → Forked by User C)

### Key Journey 4: Review Recipe History
User opens a recipe they've been refining over months → clicks "History" → sees timeline of all edits with diffs highlighting what changed in each version → can restore any previous version

---

## Technical Architecture

### Platform(s)
- **Web application** (primary — accessible from desktop, tablet, phone browser via PWA)
- **Desktop application** (Dioxus webview renderer via Wry/Tao — same WASM binary as web)
- **Mobile application** (Dioxus webview renderer — iOS WKWebView / Android WebView with WASM support; same WASM binary as web and desktop)
- **Native GPU rendering** (`dioxus-native` Blitz renderer — future consideration for true platform UI components)

All platforms share one Rust codebase. Web, desktop, and mobile webview targets compile to the same WASM binary with identical behavior (same SQLite database layer via `op-sqlite`, same SQL queries). Native GPU rendering uses a separate compilation target with native `rusqlite` instead of WASM SQLite — handled automatically by conditional compilation in our shared query-layer crate.

### Tech Stack

#### Full-Stack Framework: Dioxus Fullstack

Dioxus Fullstack is a unified Rust framework that eliminates the traditional frontend/backend split. It compiles to WebAssembly for the browser while providing server-side rendering, automatic RPC generation via `#[server]` functions, and built-in session management — all from a single codebase with shared types flowing naturally between client and server.

**What it gives us:**
- **`#[server]` functions:** Define an async function once; Dioxus auto-generates the backend endpoint AND the frontend RPC stub. Call it from WASM like any local async function — `Result<T, AppError>` propagates with zero manual serialization. This is our primary communication pattern for 90% of operations (get recipes, create versions, fork, etc.).
- **SSR + Hydration:** Components render to HTML on the server for instant first paint, then hydrate into interactive WASM in the browser. No flash of unstyled content, no client-side auth race conditions — the page is already aware of login status during SSR.
- **Unified routing:** Single `#[derive(Routable)]` enum defines both SSR page routes and API endpoints. One router, one binary.
- **Built-in session management:** Cookie-based sessions with server-side validation, integrated directly into Dioxus context providers. No separate middleware layer needed for most auth flows.
- **Asset serving:** The `asset!()` macro embeds static files (CSS, fonts, images) at compile time via linker symbols. The single binary serves them alongside SSR pages and API responses.

**What it's built on (and when we reach underneath):**
Dioxus Fullstack is built on top of **Axum** internally. We don't use Axum directly for most features — `#[server]` functions handle that. But we do reach down to raw Axum routes for a small set of cases:
- OAuth provider callbacks (`/auth/google/callback`, `/auth/apple/callback`) — external redirects land on standard HTTP endpoints
- Health checks and monitoring (`/health`) — infrastructure probes
- Webhook receivers (future) — email provider delivery receipts, etc.

These are the ~5% of routes that don't fit the server function model. Everything else flows through `#[server]`.

| Concern | Tool | Scope |
|---------|------|-------|
| UI components, routing, SSR, hydration | Dioxus core + Fullstack | All user-facing pages and interactions |
| Data mutations, queries, business logic | Dioxus `#[server]` functions | Recipe CRUD, versioning, forking, auth flows, sync operations |
| Shared database access layer | Custom `query-layer` crate (trait-based) | Compiles to both native (PostgreSQL via `sqlx`) and WASM (SQLite via `op-sqlite`) |
| Raw HTTP endpoints | Axum (via Dioxus Fullstack internals) | OAuth callbacks, health checks, infrastructure hooks |
| Schema management | Declarative `schema.sql` | Independent of any ORM — additive-only diff migrations at deploy time |

- **Learning curve consideration:** Ecosystem is maturing rapidly but not as mature as Next.js. Fewer third-party UI component libraries, but core framework is stable and well-maintained. The `#[server]` macro eliminates an entire category of boilerplate (endpoint definitions, serialization, error mapping) that would otherwise be manual work in a split-stack architecture.

#### Database: PostgreSQL
- Relational data model fits naturally (users, recipes, versions, relationships)
- JSONB columns provide flexibility for evolving recipe structures without schema migrations
- Built-in full-text search for recipe discovery and ingredient searches
- Recursive CTEs for traversing fork lineage graphs efficiently
- Potential extensions:
  - **pg_trgm** for search bar autocomplete — trigram-based infix and fuzzy string matching with GIN indexes (sub-50ms lookups)
  - **pg_search** for Elasticsearch-like keyword search, faceting, and typo tolerance without external infrastructure
  - **pgvector** for semantic search via embeddings (conceptual queries like "something quick and spicy")
  - **TimescaleDB** if we need time-series analytics on recipe engagement

#### Infrastructure & Hosting

**Railway** — App + Database hosting
- Excellent Rust support with straightforward Docker-based deployments
- Managed PostgreSQL instance (seamless connection string management)
- Auto-scaling based on traffic patterns
- Built-in environment variable management and secrets rotation
- Generous free tier for personal projects, predictable paid scaling

**Cloudflare R2** — Image storage & delivery
- S3-compatible API with zero egress fees (major cost advantage over AWS S3 for frequently-accessed recipe images)
- Global CDN edge network ensures fast image loads worldwide
- Lifecycle policies for automatic cleanup of orphaned/temporary uploads
- Integration via standard S3 SDK clients in Rust (`aws-sdk-s3` works seamlessly with R2 endpoints)

**Deployment Topology:**
```
┌───────────────────────────────────────────────┐
│              Railway Platform                   │
│                                                │
│  ┌─────────────────────────────────────────┐   │
│  │         Noms Application                 │   │
│  │                                         │   │
│  │     ┌─────────────────────────────┐    │   │
│  │     │   Dioxus Fullstack Binary   │    │   │
│  │     │                             │    │   │
│  │     │  SSR pages + WASM frontend  │    │   │
│  │     │  #[server] RPC functions    │    │   │
│  │     │  Axum routes (OAuth, health)│    │   │
│  │     └──────────────┬──────────────┘    │   │
│  └────────────────────┼───────────────────┘   │
│                       │                       │
│  ┌────────────────────▼───────────────────┐   │
│  │         PostgreSQL (Managed)            │   │
│  │  - All relational data                  │   │
│  │  - Recipe versions, fork graphs         │   │
│  │  - User accounts & social relationships │   │
│  └────────────────────────────────────────┘   │
└──────────────────────┬────────────────────────┘
                       │ S3-compatible API calls
┌──────────────────────▼────────────────────────┐
│          Cloudflare R2 Bucket                  │
│                                               │
│  /recipes/{recipe_id}/                        │
│      hero-{timestamp}.jpg                     │
│      step-1-{timestamp}.jpg                   │
│  /avatars/{user_id}/{filename}                │
└───────────────────────────────────────────────┘

CDN Edge Network (automatic for R2)
```

**Key point:** One binary handles everything. Dioxus Fullstack compiles to a single process that serves SSR-rendered HTML pages, the WASM frontend bundle, `#[server]` RPC endpoints, and a handful of raw Axum routes (OAuth callbacks, health checks). No separate API service, no CORS configuration between services, no dual-deployment pipeline.

### Architecture Notes

#### Dioxus Fullstack Communication Model
```
┌─────────────────────────────────────────────┐
│                 Browser                      │
│  ┌───────────────────────────────────────┐  │
│  │         Dioxus (WebAssembly)          │  │
│  │  - Reactive UI Components             │  │
│  │  - Client-side routing                │  │
│  │  - State management                   │  │
│  └──────────────┬────────────────────────┘  │
│                 │                            │
│   Two communication channels:               │
│                                              │
│   Channel A (95%): #[server] functions       │
│     - Auto-generated RPC                    │
│     - Typed Result<T, AppError> propagation  │
│     - Shared types between client/server    │
│                                              │
│   Channel B (5%): Raw HTTP requests          │
│     - OAuth redirects land on Axum routes    │
│     - Image uploads via presigned R2 URLs    │
│     - Health checks, webhooks               │
│                 │                             │
└─────────────────┼───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          Dioxus Fullstack Binary             │
│  ┌─────────────┐    ┌────────────────────┐  │
│  │ #[server]   │    │  Shared Business   │  │
│  │  Functions  │◄──►│  Logic & Types     │  │
│  │  (auto-     │    │  (one codebase)    │  │
│  │   generated)│    │                    │  │
│  └──────┬──────┘    └────────────────────┘  │
│         │                                   │
│  ┌──────▼───────────────────────────────┐   │
│  │        PostgreSQL                     │   │
│  │  - Recipes + Versions                 │   │
│  │  - User Graph                         │   │
│  │  - Fork Lineage DAG                   │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

**The `#[server]` pattern in practice:**
```rust
// Defined once — compiles for both server and client
#[server]
pub async fn get_recipes(owner_id: Option<Uuid>) -> Result<Vec<Recipe>, AppError> {
    // Runs on the server, accesses PostgreSQL directly
    let recipes = query_layer::list_recipes(&db_pool, owner_id).await?;
    Ok(recipes)  // Automatically serialized → sent to WASM client
}

// Called from any Dioxus component — looks like a normal async function
let recipes = match get_recipes(Some(current_user.id)).await {
    Ok(recipes) => recipes,
    Err(AppError::Unauthorized) => { /* redirect to login */ },
    Err(e) => { /* error boundary catches this */ },
};
```

No endpoint URL definitions. No manual JSON serialization. No CORS headers. The macro generates it all at compile time and the types are shared across both compilation targets.

#### Key Technical Considerations
- **Version history** requires storing full recipe snapshots or diffs at each revision — needs careful data model design. With PostgreSQL JSONB, we can store immutable version snapshots efficiently.
- **Fork chain** is a graph structure (DAG) — parent → child relationships that can branch deeply. Recursive CTEs in PostgreSQL handle traversal elegantly.
- **Community feed** will need efficient querying (potentially materialized views or event-driven updates). Consider denormalizing follow counts and recent activity for performance.
- **URL scraping/parsing** for recipe import requires handling diverse website structures, possibly leveraging schema.org structured data. Rust's `select` crate provides excellent HTML parsing capabilities.

#### Error Handling Strategy — Maximum Visibility in Development, Graceful Degradation in Production

Errors are inevitable. Our philosophy is that every error should be **impossible to ignore during development** and **gracefully handled for users in production**. We design for failures at every layer of the stack with escalating visibility.

##### Rust's Exhaustiveness Guarantee — What It Enforces and Where We Must Be Disciplined

Rust provides compile-time exhaustiveness checking, but only for certain patterns:

| Pattern | Compiler Enforcement | Risk |
|---------|---------------------|------|
| `match` on `enum` variants | ✅ **Hard stop** — missing arms produce a compile error | Zero risk if used correctly |
| `Result<T, E>` via `?` propagation | ⚠️ Error bubbles up to caller — someone must handle it eventually | Deferred handling is fine; lost errors are not |
| `.unwrap()` / `.expect()` | ❌ **Runtime panic** — no compile-time safety | Crashes the entire thread with minimal context if misused |
| `if let Some(x) = opt` | ❌ Partial match — silently ignores unmatched variants | Easy to forget the `None` case entirely |

**Our rule:** `.unwrap()` is banned everywhere except test code and one-shot initialization in `main()`. Enforced via Clippy deny configuration:
```toml
# .clippy.toml or cargo clippy args
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
unreachable = "deny"
```

Every function either handles its error explicitly via `match`/`if-let-else`, propagates it with `?`, or converts it into a user-friendly variant using `.map_err()`. There are no silent drops.

##### Layered Error Architecture — Four Boundaries, Escalating Visibility

Errors propagate outward through four layers. Each layer adds context, logs the error, and decides how to present it based on whether we're in development or production mode.

###### Layer 1: Query / Data Access Layer

All database operations return `Result<T, AppError>` with a structured error enum — never raw driver errors leaking into application code:

```rust
// Shared across both PostgreSQL (native) and SQLite (WASM) backends
#[derive(Debug)]
pub enum AppError {
    NotFound(String),           // "Recipe not found" — resource doesn't exist
    DbError(DatabaseError),     // Wrapped driver error with original message preserved
    ValidationError(Vec<String>), // Multiple field-level validation failures
    Unauthorized,               // Missing or expired session
    Forbidden,                  // Authenticated but no permission for this action
    Conflict(String),           // e.g., "Recipe already exists with this title"
    ExternalService {          // Third-party service failure (R2 presigned URL generation, etc.)
        service: String,
        status: u16,
        message: String,
    },
    Internal(String),          // Catch-all for truly unexpected failures — should be rare
}

impl AppError {
    /// Human-readable message suitable for display to end users (production-safe)
    pub fn user_message(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "The requested resource was not found.",
            Self::DbError(_) | Self::Internal(_) => "Something went wrong. Please try again later.",
            Self::ValidationError(_) => "Please check your input and try again.",
            Self::Unauthorized => "Please sign in to continue.",
            Self::Forbidden => "You don't have permission to do that.",
            Self::Conflict(_) => "This action conflicts with existing data.",
            Self::ExternalService { .. } => "An external service is temporarily unavailable.",
        }
    }

    /// Full diagnostic string — includes raw error details, file:line context, original messages.
    /// ONLY used in development mode. Never sent to clients in production.
    pub fn debug_details(&self) -> String {
        format!("{:#?}", self)  // Debug-formatted with full chain of wrapped errors
    }
}

// Implement std::error::Error for ? propagation and thiserror integration
impl std::fmt::Display for AppError { /* ... */ }
impl std::error::Error for AppError { /* ... */ }
```

Every query function in the shared `query-layer` crate returns `Result<T, AppError>`. Database driver errors (`sqlx::Error`, `op-sqlite::Error`) are wrapped into `AppError::DbError` at the boundary — application code never sees raw driver types. This gives us one error type to reason about across both backends.

###### Layer 2A: Dioxus `#[server]` Functions (Primary Pattern — ~95% of calls)

Server functions are the primary communication channel between WASM frontend and server backend. They propagate typed errors directly — no manual JSON serialization, no HTTP status code mapping, no CORS headers. The `#[server]` macro handles all of it at compile time.

```rust
// Server function — runs on the backend, called from WASM like a normal async fn
#[server]
pub async fn create_recipe(
    title: String,
    ingredients: serde_json::Value,
) -> Result<RecipeResponse, AppError> {
    // If this returns Err(AppError::...), Dioxus Fullstack automatically:
    // 1. Serializes the error (AppError must implement Serialize + Deserialize)
    // 2. Sends it back to the WASM client over the RPC channel
    // 3. The caller receives Result<RecipeResponse, AppError> — same type on both sides

    let recipe = query_layer::insert_recipe(&db_pool, title, ingredients).await?;
    Ok(RecipeResponse { id: recipe.id, title: recipe.title })
}

// Frontend calls it like any async function — typed error flows directly:
match create_recipe("Lasagna".into(), ingredients_json).await {
    Ok(recipe) => /* success */,
    Err(AppError::ValidationError(fields)) => {
        // Handle validation errors inline (show form field messages)
        show_form_errors(fields);
    }
    Err(AppError::Unauthorized) => {
        // Redirect to login
        navigate_to("/login");
    }
    Err(e) => {
        // Unexpected error — let the error boundary catch it and show modal
        return Err(e.into());  // Propagates up to component's ErrorBoundary
    }
}
```

**Error logging for server functions:** Every `#[server]` function logs errors before returning them. We wrap this in a helper so every server function has consistent error visibility:

```rust
// Helper — call at the top of every #[server] function's error handling path
#[cfg(debug_assertions)]
fn log_server_error(error: &AppError, context: &str) {
    eprintln!(
        "[SERVER ERROR @ {}] {}: {}\nDetails: {}",
        chrono::Local::now().format("%H:%M:%S%.3f"),
        context,
        error.user_message(),
        error.debug_details()
    );
}

#[cfg(not(debug_assertions))]
fn log_server_error(error: &AppError, context: &str) {
    tracing::error!(
        context = %context,
        error_code = ?error,
        user_message = %error.user_message(),
        debug = %error.debug_details(),
        "Server function error"
    );
}

// Usage in a server function:
#[server]
pub async fn fork_recipe(source_id: Uuid, message: String) -> Result<RecipeResponse, AppError> {
    match query_layer::fork_recipe(&db_pool, source_id, &message).await {
        Ok(recipe) => Ok(RecipeResponse { .. }),
        Err(e) => {
            log_server_error(&e, "fork_recipe");  // Always log, dev or prod
            Err(e)
        }
    }
}
```

In **development mode**, every server function error prints to the Railway terminal with full debug details, timestamp, and context label. In **production**, structured `tracing` logs capture everything silently — no diagnostic data leaks to the client beyond what `AppError::user_message()` exposes.

###### Layer 2B: Raw Axum Routes (Fallback Pattern — ~5% of calls)

A small set of routes don't fit the server function model: OAuth provider callbacks, health checks, webhook receivers. These use standard Axum handlers with manual HTTP response construction:

```rust
// Standardized API error response shape — used only by raw Axum endpoints
#[derive(serde::Serialize)]
struct ApiErrorResponse {
    pub error: String,        // Machine-readable code (e.g., "not_found")
    pub message: String,      // Human-readable explanation (production-safe)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,  // Dev-only diagnostics; omitted in production
}

// Axum handler — converts AppError → HTTP response automatically
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code) = match &self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            Self::ValidationError(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_error"),
            Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::ExternalService { .. } => (StatusCode::BAD_GATEWAY, "external_service_error"),
            Self::DbError(_) | Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        // LOG EVERY ERROR — maximum visibility in development
        #[cfg(debug_assertions)]
        eprintln!(
            "[AXUM ERROR] {} ({}) — {}\nDetails: {}",
            status,
            error_code,
            self.user_message(),
            self.debug_details()
        );

        #[cfg(not(debug_assertions))]
        tracing::error!(
            layer = "axum_route",
            error_code = %error_code,
            user_message = %self.user_message(),
            debug = %self.debug_details(),
            "Axum route error"
        );

        let body = ApiErrorResponse {
            error: error_code.to_string(),
            message: self.user_message().to_string(),
            details: if cfg!(debug_assertions) { Some(self.debug_details()) } else { None },
        };

        (status, Json(body)).into_response()
    }
}
```

**Top-level panic catcher:** A Dioxus Fullstack / Axum middleware layer wraps every request in a `catch_unwind` boundary. If any handler or server function panics (which shouldn't happen with our error handling discipline), the middleware catches it, logs the full stack trace to stderr/tracing, and returns a sanitized 500 response:

```rust
// Middleware — catches unwinding panics that escaped all error handling layers
async fn panic_catcher(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let future = next.run(req);
    match futures::FutureExt::catch_unwind(send_wrapper(future)).await {
        Ok(response) => response,
        Err(panic_info) => {
            #[cfg(debug_assertions)]
            eprintln!("[PANIC] Unhandled panic caught by middleware:\n{:?}", panic_info);

            #[cfg(not(debug_assertions))]
            tracing::error!(panic = ?panic_info, "Unhandled panic — full stack logged to server");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiErrorResponse {
                    error: "internal_error".to_string(),
                    message: "Something unexpected went wrong.".to_string(),
                    details: None,  // Never expose panic internals in production
                }),
            )
        }
    }
}
```

This is the **safety net** — it should never fire if Layers 1-2A are working correctly. But if a `panic!()` or unhandled `.unwrap()` slips through (e.g., during development before Clippy catches it), this middleware ensures we still get full diagnostic output and return a valid HTTP response instead of crashing the connection silently.

###### Layer 3: Dioxus WASM Frontend — Error Boundaries & Dev Modals

Dioxus provides `<ErrorBoundary>` components that catch panics within their child subtree and render fallback UI instead of crashing the entire app. We use them at three granularity levels:

| Boundary Level | Scope | Fallback Behavior (Dev Mode) | Fallback Behavior (Production) |
|---|---|---|---|
| **App-level** (`<ErrorBoundary>` around `<Router>`) | Entire application | Full-screen error screen with title, message, full debug details, "Reload" button. Logged to browser console. | Minimal full-screen apology: *"We're having trouble loading Noms. Please check back shortly."* No raw details exposed. |
| **Page-level** (each route wrapped in `<ErrorBoundary>`) | Single page/component tree | Modal overlay with error title, user message, expandable debug section showing stack trace + browser console log. Rest of app shell (nav bar) stays functional. | Inline toast or banner: *"Something went wrong on this page."* Page shows empty/error state; navigation still works. |
| **Component-level** (critical interactive components like Recipe Editor, Fork Graph) | Single component | Component area renders error card with message + details. Surrounding UI unaffected. | Component area renders subtle placeholder: *"Couldn't load — try refreshing."* |

**Development Error Modal:** When an error boundary catches a panic in dev mode, it renders a modal directly on screen (not just console):

```rust
// Development-only error modal component
#[component]
fn DevErrorModal(error: String, stack_trace: Option<String>) -> Element {
    rsx! {
        // Fixed overlay — blocks interaction until dismissed
        div { class: "fixed inset-0 z-[9999] bg-black/60 backdrop-blur-sm flex items-center justify-center p-4",

            // Modal card
            div { class: "bg-surface shadow-neumo-card rounded-2xl max-w-2xl w-full max-h-[85vh] overflow-auto",

                // Header — clear visual signal that something broke
                div { class: "border-b border-error/30 p-4 flex items-center gap-3",
                    div { class: "w-10 h-10 rounded-full bg-error/20 flex items-center justify-center text-error", "⚠" }
                    div {
                        h2 { class: "text-text-primary font-bold text-lg", "Application Error" }
                        p { class: "text-text-secondary text-sm", error.clone() }
                    }
                }

                // Body — user-friendly explanation first, technical details expandable
                div { class: "p-4 space-y-4",

                    p { class: "text-text-primary",
                        "An unexpected error occurred while rendering this component. \
                         The app should still be functional. Check the browser console for full details."
                    }

                    // Expandable debug section — stack trace, file locations, etc.
                    details { class: "bg-bg-base rounded-lg p-3",
                        summary { class: "text-accent cursor-pointer font-medium", "Debug Details" }
                        pre { class: "mt-2 text-xs text-text-secondary overflow-auto max-h-64 bg-black/5 rounded p-2",
                            stack_trace.as_deref().unwrap_or("No stack trace available")
                        }
                    }

                    // Action buttons
                    div { class: "flex gap-3 mt-4",
                        button {
                            class: "px-4 py-2 bg-accent text-white rounded-lg hover:bg-accent-hover transition-colors",
                            onclick: move |_| { /* copy error details to clipboard */ },
                            "📋 Copy Details"
                        }
                        button {
                            class: "px-4 py-2 border border-text-tertiary rounded-lg hover:border-text-secondary transition-colors",
                            onclick: move |_| { window.location.reload(); },
                            "↻ Reload Page"
                        }
                    }
                }
            }
        }
    }
}
```

**Console logging in WASM:** Every error boundary also logs to the browser console using `web_sys::console::error_1()` with formatted output including timestamp, error message, and stack trace. This ensures developers checking DevTools see everything — the modal is just a convenient on-screen supplement.

```rust
// Helper used by all error boundaries in dev mode
#[cfg(debug_assertions)]
fn log_error_to_console(message: &str, details: Option<&str>) {
    use web_sys::console;
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let full_msg = format!("[NOMS ERROR @ {}] {}", timestamp, message);

    console::error_1(&full_msg.into());

    if let Some(details) = details {
        console::warn_1(&format!("  ↳ Details:\n{}", details).into());
    }
}
```

###### Layer 4: Production Error Tracking (Future)

In production, raw error details are never sent to the client. Instead, they're reported silently to an error tracking service (Sentry, Honeybadger, or GlitchTip) via a background API call from both Axum middleware and Dioxus error boundaries. This gives us full diagnostic visibility without exposing internals to end users.

The plumbing for this is straightforward — each layer already logs structured data; we just add an HTTP POST to the tracking service alongside existing `tracing`/`console.error` calls, gated behind a feature flag or environment variable:

```rust
// Pseudocode — both Axum and Dioxus error handlers call this in production
#[cfg(all(not(debug_assertions), feature = "error-tracking"))]
async fn report_error_to_sentry(error: &AppError) {
    // POST to Sentry/Honeybadger/GlitchTip with full context
    // Runs async, fire-and-forget — never blocks user-facing response
}
```

##### Summary Table — Error Visibility by Mode

| Layer | Development | Production |
|-------|-------------|------------|
| **Layer 1: Query layer** | Full debug output via `AppError::debug_details()` in every error path | Same internal logging; details wrapped and sanitized before leaving the crate |
| **Layer 2A: `#[server]` functions** (95% of calls) | Terminal stderr with timestamp + context label + full debug details. Typed `AppError` propagates directly to WASM caller — no serialization boilerplate. | Structured `tracing` logs via `log_server_error()` helper. Client receives only the typed `AppError` variant — no raw internals exposed. |
| **Layer 2B: Raw Axum routes** (5% of calls) | Terminal stderr + JSON response includes `details` field with full diagnostic string | Structured `tracing` logs; JSON response omits `details`. Panic middleware catches anything that escaped. |
| **Layer 3: Dioxus UI error boundaries** | On-screen error modal with expandable debug section + browser console.log | Inline toast/banner with generic message. Full details sent silently to error tracking service. |
| **Layer 4: App-level crash (panic)** | Full-screen dev error screen with reload button + panic middleware logs full stack | Minimal apology screen — no raw data exposed. Panic already logged server-side and reported to tracking. |

### Funding Principles

Noms is built to remain accessible and community-driven. These are the non-negotiable guardrails that any future monetization must respect:

- **No paywalls.** All core features — recipe creation, discovery, sharing, commenting, following — remain free for every user indefinitely. No premium tiers, no gated functionality.
- **No data selling.** User behavior, recipes, and community activity are never sold to third parties. Period. Analytics (if any) are aggregate and anonymous.
- **No intrusive ads.** The platform stays clean. No programmatic ad networks, no banner farms, no interstitials that interrupt the cooking experience.
- **Opt-in support only.** Any revenue-generating mechanism is explicitly chosen by the user — whether that's a donation, enabling contextual sponsor content, or shopping ingredients through an affiliate link. Nothing happens silently in the background.
- **Partnership-driven growth.** Long-term sustainability comes from curated partnerships with brands and services relevant to cooking (grocery delivery, kitchen tools, local food producers) rather than extracting value from users directly.

These principles are structural — they shape how we build features now so that future monetization layers can be added without retrofitting or breaking trust.

### Testing Strategy

We follow a **test-first** approach: tests describe expected behavior before implementation begins. This ensures requirements are concrete and verifiable, not aspirational. Every feature starts with a failing test — the implementation is complete when it passes.

#### Test Pyramid

```
         ┌─────────────┐
         │   E2E (5%)   │  Playwright — critical user journeys only
         ├─────────────┤
     ┌───│ Integration │───┐
     │   │  (20-30%)    │   │  Testcontainers + server functions + auth flows
     │   ├─────────────┤   │
     │   │    Unit      │   │
     │   │  (65-70%)    │   │  Pure logic, business rules, component rendering
     └───┴─────────────┘───┘
```

The pyramid is intentional: most tests are fast unit-level assertions at the bottom, a moderate layer of database-aware integration tests in the middle, and a thin E2E safety net on top. This keeps `cargo test` fast for daily development while still covering the full stack.

#### Layer 1 — Unit Tests (Fast, Everywhere)

Unit tests live inline with source code using Rust's built-in `#[cfg(test)]` modules at the bottom of each file. They require zero external dependencies and run in milliseconds.

**What to test:**
- **Business rules**: "Can a user follow themselves?" → no. "Does recipe visibility respect private flag?" → yes/no based on owner + permissions.
- **Validation functions**: Username format, ingredient parsing, slug generation, input sanitization.
- **Data transformations**: Recipe serialization/deserialization, pagination math, search result ranking logic.
- **Component rendering** (Dioxus): Use `dioxus_ssr::render_element()` to verify components produce the expected HTML structure for given props — no browser needed, just VirtualDom + SSR output comparison.

```rust
// Business rule test: inline with source code
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_follow_self() {
        let user_id = Uuid::new_v4();
        assert!(validate_follow_request(user_id, user_id).is_err());
    }

    // Component rendering test via SSR — no browser required
    #[test]
    fn recipe_card_shows_author_and_title() {
        let element = rsx! { RecipeCard { recipe: sample_recipe(), author: "Sam" } };
        let html = dioxus_ssr::render_element(element);
        assert!(html.contains("Sourdough Bread"));
        assert!(html.contains("by Sam"));
    }
}
```

**Run:** `cargo test` — parallel by default, completes in seconds for the entire workspace. This is the primary feedback loop and should pass on every file save.

#### Layer 2 — Integration Tests (Database + Server Functions)

Integration tests live in a `tests/` directory at the crate root. They spin up ephemeral PostgreSQL containers via **testcontainers-rs** so each test gets a fresh, isolated database with zero cross-contamination between runs.

```
crates/backend/tests/
├── auth_flow.rs       # OAuth login → session creation → protected route access
├── recipe_crud.rs     # Full create → read → update → delete cycles
└── follow_graph.rs    # Multi-user scenarios: following, unfollowing, feed generation
```

**What to test:**
- **Server function contracts**: Call the actual `#[server]` function through its RPC interface and assert response type + data integrity. This catches serialization mismatches between client and server.
- **Database round-trips**: Insert → query → update → delete against real PostgreSQL. Catches SQL bugs, constraint violations, missing indexes that unit tests can't see.
- **Authentication flows**: Full login via OAuth mock → session creation → protected route access → logout. End-to-end auth verification, not just token parsing in isolation.
- **Multi-user scenarios**: Two users following each other, recipe sharing permissions (public vs private visibility), concurrent edit handling.

```rust
// crates/backend/tests/recipe_crud.rs
#[tokio::test]
async fn create_recipe_persists_and_returns_id() {
    let db = PostgresContainer::start().await;

    // Run schema migration against fresh container
    migrate_to(&db.url()).await.unwrap();

    let recipe = create_recipe(db.pool(), CreateRecipeInput { title: "Sourdough Bread", ... }).await.unwrap();
    assert!(recipe.id.is_some());

    // Verify persistence — read back from database independently of the creation path
    let fetched = get_recipe_by_id(db.pool(), recipe.id).await.unwrap();
    assert_eq!(fetched.title, "Sourdough Bread");
}
// `db` dropped here → container destroyed automatically (RAII)
```

**Performance:** Container startup adds ~1-2 seconds per test. But since each test gets its own isolated container and `cargo test` parallelizes freely across CPU cores, the wall-clock time for a full integration suite is typically 5-10 seconds. In CI we can reuse containers across related tests via **test-containers-util** to reduce overhead further — it creates one shared PostgreSQL instance and provisions a separate database per test automatically.

#### Snapshot Testing with Insta

For any function that produces structured output (JSON API responses, rendered HTML, serialized recipe data), we use **[insta](https://crates.io/crates/insta)** for snapshot testing instead of hand-written assertions:

```rust
#[test]
fn recipe_response_matches_contract() {
    let json = serde_json::to_string_pretty(&sample_recipe_response()).unwrap();
    insta::assert_snapshot!(json);
}
```

Insta stores expected output in `.snap` files alongside the test source. When output changes, `cargo insta review` opens an interactive terminal UI showing clean diffs — you accept or reject each change individually. In CI (where the `CI` environment variable is set), any uncommitted snapshot change causes the build to fail automatically.

```bash
# Local workflow:
cargo insta test    # Run tests, write .snap.new files for any changes
cargo insta review  # Interactive TUI — accept/reject each diff
git add *.snap      # Commit accepted snapshots alongside code changes

# CI behavior (automatic): fails if any snapshot is out of date
```

#### Layer 3 — E2E Tests (Browser, Minimal Set)

End-to-end tests use **Playwright** to drive a real browser against the running application. They're slow and fragile by nature — keep them to critical user journeys only, never every button click.

**What to test:**
- **Authentication flow**: Visit login page → OAuth redirect → land on dashboard (the full happy path with SSR hydration).
- **Recipe creation end-to-end**: Fill form → submit → verify recipe appears in feed with correct metadata.
- **Core navigation**: Route changes work, no blank screens, SPA transitions are smooth.

That's it. Everything else should be covered by layers 1 and 2. E2E tests are the safety net for things that only break when all layers interact through a real browser — hydration mismatches, CSS layout issues, JavaScript event binding failures.

```javascript
// playwright-tests/auth.spec.js
test('login flow lands on dashboard', async ({ page }) => {
  await page.goto('http://localhost:8080/login');
  // ... OAuth mock interaction
  await expect(page.locator('[data-testid="dashboard"]')).toBeVisible();
});
```

#### Test-First Workflow

The test-first approach is about **describing the contract before building it**, not about writing every assertion before every line of code:

```
Feature design (DESIGN.md or roadmap issue)
    ↓
Write failing test(s) that describe expected behavior
    ↓
  cargo test → FAILS ✓ (confirms the gap exists, test is valid)
    ↓
Implement minimum code to make it pass
    ↓
  cargo test → PASSES ✓
    ↓
Refactor with confidence — tests catch regressions automatically
```

For UI components, that means: "Given these props, the rendered HTML contains X" — written as an SSR assertion before you build the component. For server functions: "Given this input and database state, the response is Y" — written as an integration test against a fresh container before touching production queries. The failing test is the specification; passing it is the implementation.

#### CI Automation — When Each Layer Runs

| Test Layer | Local (every save) | Every PR/commit | On `main` merge |
|------------|-------------------|-----------------|------------------|
| **Unit tests** (`cargo test`) | ✅ Yes | ✅ Yes | ✅ Yes |
| **Integration tests** (with containers) | ✅ Yes | ✅ Yes | ✅ Yes |
| **WASM build verification** (`dx build --release`) | As needed | ✅ Yes | ✅ Yes |
| **E2E tests** (Playwright) | Manual only | ❌ No — too slow for PR feedback | ✅ Yes — final safety gate before staging deploy |

Unit and integration tests run on every push — fast feedback loop. Full E2E runs only when code hits `main` to keep CI times reasonable while still catching browser-specific breakages before they reach the staging deployment. The result: PRs get test results in under a minute, merges trigger the full suite as a final gate.

### CI/CD & Deployment Strategy

We will split responsibilities cleanly: **GitHub Actions** handles pure Continuous Integration (linting, testing, build verification), while **Railway's native deployment triggers** handle all Continuous Delivery. This eliminates redundant token management and leverages Railway's built-in build caching.

#### 1. Environment Separation
To ensure zero risk of bleeding test data into production or breaking live users during schema changes, we maintain two completely isolated Railway projects:

| Feature | Staging (`noms-staging`) | Production (`noms-prod`) |
| :--- | :--- | :--- |
| **Trigger** | Automatic on every `git push` to `main` branch | Manual promotion via Git Tags (`v0.1.0`, etc.) |
| **Database** | Fresh, disposable PostgreSQL instance (can be wiped/recreated at will) | Persistent, heavily backed-up PostgreSQL with strict migration safety |
| **Domain** | `noms-staging.up.railway.app` | `app.noms.com` (custom domain later) |
| **Secrets** | Dummy OAuth clients, test R2 buckets | Live Google/Apple/GitHub OAuth IDs, production R2 credentials |

#### 2. CI Pipeline (Continuous Integration via GitHub Actions)
Runs on every Pull Request and push to `main`. If any step fails, the branch is blocked from merging. This keeps our CD completely decoupled from quality gates.

```yaml
# .github/workflows/ci.yml
name: Noms CI
on: [push, pull_request]

jobs:
  lint-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Linting & Formatting
      - name: Rustfmt
        run: cargo fmt --all --check
      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

      # Unit & Integration Tests (Backend)
      - name: Run Backend Tests
        run: cargo test --workspace

      # WASM Build Verification (Frontend)
      - name: Setup Dioxus Toolchain
        run: rustup target add wasm32-unknown-unknown && cargo install dx
      - name: Verify WASM Build
        run: dx build --release

  # Schema safety gate — runs only when schema.sql changes
  schema-check:
    runs-on: ubuntu-latest
    if: |
      github.event_name == 'pull_request' &&
      github.event.pull_request.added != null ||
      github.event.pull_request.removed != null ||
      github.event.pull_request.changed != null
    steps:
      - uses: actions/checkout@v4

      # Only run if migrations/schema.sql was modified in this PR
      - name: Check if schema changed
        id: schema-changed
        run: |
          if git diff --name-only "${{ github.event.pull_request.base.sha }}..HEAD" \
             | grep -q "migrations/schema\.sql"; then
            echo "changed=true" >> $GITHUB_OUTPUT
          else
            echo "changed=false" >> $GITHUB_OUTPUT
          fi

      - name: Install pgmold
        if: steps.schema-changed.outputs.changed == 'true'
        run: cargo install pgmold --locked

      - name: Plan migration against staging DB
        if: steps.schema-changed.outputs.changed == 'true'
        env:
          DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
        run: |
          pgmold plan \
            -s sql:migrations/schema.sql \
            -d "$DATABASE_URL" \
            --json > /tmp/migration-plan.json

          # Print human-readable plan for PR review context
          echo "### Migration Plan" >> $GITHUB_STEP_SUMMARY
          jq -r '.statements[]?.sql // empty' /tmp/migration-plan.json \
            | while read -r sql; do echo "\`\`\`sql\n$sql\n\`\`\`" >> $GITHUB_STEP_SUMMARY; done

      - name: Reject destructive changes
        if: steps.schema-changed.outputs.changed == 'true'
        run: |
          DROPS=$(jq '.summary.drops // 0' /tmp/migration-plan.json)
          RENAMES=$(jq '.summary.renames // 0' /tmp/migration-plan.json)
          if [ "$DROPS" -gt 0 ] || [ "$RENAMES" -gt 0 ]; then
            echo ""
            echo "❌ Destructive schema changes detected:"
            echo "   Drops: $DROPS, Renames: $RENAMES"
            echo "   Use the additive pattern instead (add new column → backfill → update code → later drop old)."
            exit 1
          fi

      - name: Post migration plan as PR comment
        if: steps.schema-changed.outputs.changed == 'true'
        uses: marocchino/sticky-pull-request-comment@v2
        with:
          path: /tmp/migration-plan.json
```

#### 3. CD Strategy (Railway Native Deployments)

**Staging Deployment (`main` branch):**
We connect the `noms-staging` Railway project directly to our GitHub repository. In the Railway UI, we configure a **Branch Deployment** rule pointing at `main`. Every time code is merged to main, Railway automatically:
1. Checks out the commit and builds the Rust binary + WASM artifacts using its cached Docker layers.
2. Runs database migrations via a pre-deploy script (see below).
3. Spins up the new version behind a health check before routing live traffic.

**Production Deployment (Git Tags):**
In the `noms-prod` Railway project, we configure a **Tag Deployment** rule. Production is never deployed automatically. To release a new version, we create an annotated Git tag locally or via GitHub Releases (`git tag v0.1.0 && git push origin --tags`). Railway detects the tag and deploys it to production with full migration support.

#### 4. Database Migration Strategy — Declarative Schema, Non-Destructive by Default

We will adopt a **declarative, state-based** database management strategy using a single `schema.sql` file that represents the exact, complete desired state of our PostgreSQL database at all times. There are no incremental migration scripts to hunt through — just one source of truth. We use **[pgmold](https://github.com/fmguerreiro/pgmold)** (Rust-native) as our diff-and-apply engine.

```
migrations/
├── schema.sql       # Single source of truth — complete desired database state
└── seed.sql         # Reference data (tags master list, default settings) — idempotent inserts
```

##### Why pgmold

pgmold is a Rust-native CLI that performs Terraform-style declarative schema management for PostgreSQL. It takes our `schema.sql` as input, diffs it against the live database, and generates safe migration SQL automatically. Key properties:

- **Rust-native** — single binary via `cargo install pgmold`, no Go/JVM dependency
- **Native SQL format** — no HCL or DSL; standard PostgreSQL DDL is the schema language
- **Safety linting built-in** — destructive operations blocked by default with a configurable safety layer
- **Production mode** — blocks table drops entirely; lock hazard warnings prevent downtime-causing changes
- **Transactional apply** — all migrations run in a single transaction; if anything fails, nothing changes
- **JSON output** — machine-readable plan for CI gates (`--json` flag)
- **Drift detection** — `pgmold drift` compares live DB against schema and exits non-zero on mismatch
- **PostgreSQL-only focus** — handles RLS policies, partitioned tables, grants, row-level security as first-class citizens

##### How It Works — Three Phases Per Deployment

**Phase 1: Plan** — pgmold introspects the live database, compares it against `schema.sql`, and computes the exact delta. Output is a dependency-ordered set of migration statements with lock hazard annotations.

```bash
# Preview what would change (non-destructive only):
pgmold plan \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL"
```

**Phase 2: Gate** — By default, pgmold's safety layer blocks destructive operations (`DROP TABLE`, `DROP COLUMN`, type narrowing). If the plan contains any blocked operation, it exits non-zero with a clear error listing exactly what was rejected. In CI we parse the JSON output to enforce this programmatically.

```bash
# JSON plan for automated gating:
pgmold plan \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL" \
  --json > /tmp/migration-plan.json

# Fail if any destructive changes exist:
if jq -e '.summary | select(.drops > 0 or .renames > 0)' /tmp/migration-plan.json; then
    echo "❌ Destructive schema changes detected — use additive pattern instead" >&2
    exit 1
fi
```

**Phase 3: Apply** — Approved statements are executed in a single transaction. All-or-nothing: if any statement fails, the entire migration rolls back and the previous version remains live. pgmold also handles idempotency internally (`CREATE TABLE IF NOT EXISTS`, `IF NOT EXISTS` on indexes, etc.) so re-running the same plan is safe.

```bash
# Apply non-destructive migrations (default behavior):
pgmold apply \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL"
```

##### Allowed vs Blocked Operations

pgmold operates strictly on **DDL** (schema structure — tables, columns, indexes, constraints). It never executes arbitrary data-level statements (**DML**: `DELETE`, `TRUNCATE`, bulk `UPDATE`). DML outside of seed scripts is not part of the migration pipeline at all. This section covers both categories:

| Operation | Status | Rationale |
|-----------|--------|-----------|
| `CREATE TABLE` | ✅ **Allowed** | New table, no existing data affected. pgmold uses `IF NOT EXISTS`. |
| `ALTER TABLE ADD COLUMN` (with safe default) | ✅ **Allowed** | Adds optional column. Existing rows get the default value (`NULL`, `''`, `FALSE`). Zero downtime. |
| `ALTER TABLE ADD CONSTRAINT` (foreign key, check) | ✅ **Allowed** | Validates existing data on apply; if validation fails, deployment halts before any change is committed. Safe fail-fast. pgmold uses online constraint building (`NOT VALID` then `VALIDATE`) to avoid table locks. |
| `CREATE INDEX` | ✅ **Allowed** | Purely additive performance improvement. Uses `IF NOT EXISTS`. Built concurrently (`CONCURRENTLY`) to avoid table locks. |
| `ALTER TABLE ALTER COLUMN SET DEFAULT` | ✅ **Allowed** | Changes default for future inserts only; existing data untouched. |
| `ALTER TABLE ADD FOREIGN KEY` | ✅ **Allowed** | Validates referential integrity on apply; safe fail-fast if orphaned rows exist. |
| Column type widening (`VARCHAR(50)` → `VARCHAR(200)`) | ✅ **Allowed** | Existing values fit in the new width. No data loss possible. |
| `INSERT` (seed data via upsert) | ✅ **Allowed** | Idempotent reference data using `ON CONFLICT DO UPDATE`. Runs after schema migration from `seed.sql`. Safe to repeat. |
| `DROP TABLE` | ❌ **Blocked** | Irreversible data destruction. Requires manual destructive override (see below). pgmold's production mode blocks this entirely by default. |
| `DROP COLUMN` | ❌ **Blocked** | Same as above — data is gone forever. Column remains until explicit cleanup. |
| `ALTER TABLE DROP CONSTRAINT` | ❌ **Blocked** | Removes safety guarantees. If the constraint is unwanted, leave it; add a new one if needed. |
| `ALTER COLUMN TYPE` (narrowing or incompatible) | ❌ **Blocked** | `INTEGER` → `SMALLINT` could truncate data. `TEXT` → `VARCHAR(10)` could reject existing rows. pgmold flags this as a lock hazard and blocks it under our safety config. |
| **Column / Table renames** (`RENAME TO`) | ❌ **Blocked** | Non-destructive at the database level, but creates a coordination gap: every query, struct field, and JOIN referencing the old name must change atomically with the rename. If you miss one reference (and there are always missed references), your app runs silently against a schema it doesn't know about. And because drops are blocked, the old column can't be cleaned up to force discovery of broken queries. Result: two columns coexisting indefinitely with nobody sure which is canonical. |
| `DELETE FROM ...` | ❌ **Not in scope** | pgmold does not execute DML statements. Data deletion happens through application code (API endpoints, server functions), never through migration scripts. If you need to clean up data during a transition, write an idempotent application-level script — don't bake `DELETE` into the schema pipeline. |
| `TRUNCATE TABLE ...` | ❌ **Not in scope** | Same as above. Truncating a table wipes every row instantly with no undo. This is strictly a local-development-only operation (see `local-reset-db.sh`). Never appears in migration or seed scripts. |
| Bulk `UPDATE ... SET ... WHERE ...` | ⚠️ **Manual only** | Data backfills during column transitions (e.g., populating a new column from an old one) are one-time operations run manually or via a server function — not part of the automated pipeline. They're safe when scoped correctly, but should never live in `schema.sql` or `seed.sql`. |

##### Why Renames Are Blocked — Use Additive Instead

To "rename" a column safely under this policy, follow the additive pattern:

```
Step 1: Deploy new column alongside old column (in schema.sql):
  ALTER TABLE recipes ADD COLUMN display_name VARCHAR(200);
  → pgmold apply (non-destructive, safe)

Step 2: Backfill data (one-time migration script, run manually or via server function):
  UPDATE recipes SET display_name = title WHERE display_name IS NULL;

Step 3: Update all application code to reference `display_name` instead of `title`:
  Deploy the code change — old column still exists, nothing breaks.

Step 4: Verify everything works against `display_name` in production:
  Run queries manually, check logs, monitor error rates.

Step 5: Later — request manual destructive override to drop `title`:
  Two-person approval, explicit opt-in, audit trail.
```

This path is more steps but eliminates the coordination gap entirely. At every intermediate point, both column names work and nothing breaks until you explicitly choose to remove the old one.

##### Manual Destructive Override — When You Actually Need to Drop Something

Sometimes a column has served its purpose and needs cleanup. We support this via an **explicit opt-in** using pgmold's `--allow-destructive` flag:

```bash
# Normal deployment (non-destructive only) — this is the default:
pgmold apply \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL"

# Destructive override — explicit opt-in, must be run manually:
pgmold apply \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL" \
  --allow-destructive \
  --production-mode=false
```

The destructive override path requires:
1. **Two-person approval** — the deploying developer gets explicit sign-off from another team member via a shared channel (e.g., Slack/PR comment)
2. **Audit log** — pgmold's JSON plan is captured and appended to `docs/migration-audit.md` before execution for traceability:

```bash
# Capture audit trail before applying destructive changes:
pgmold plan \
  -s sql:migrations/schema.sql \
  -d "$DATABASE_URL" \
  --json > /tmp/destructive-plan.json

echo "## $(date -u +%Y-%m-%dT%H:%M:%SZ) — Destructive Migration by @<author>" >> docs/migration-audit.md
jq '.' /tmp/destructive-plan.json >> docs/migration-audit.md
echo "" >> docs/migration-audit.md
```

This ensures destructive migrations are: **intentional, visible, auditable, and rare.** They happen at most a handful of times per project lifecycle, not on every deploy.

##### CI Gate — Catch Destructive Changes Before Merge

The GitHub Actions CI pipeline includes a pgmold-based schema check that runs on every pull request modifying `migrations/schema.sql`:

```yaml
  # Schema safety gate in .github/workflows/ci.yml
  - name: Install pgmold
    run: cargo install pgmold --locked

  - name: Check Schema for Destructive Changes
    env:
      DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
    run: |
      pgmold plan \
        -s sql:migrations/schema.sql \
        -d "$DATABASE_URL" \
        --json > /tmp/migration-plan.json

      # Print human-readable plan for PR review context
      cat /tmp/migration-plan.json | jq '.statements[]?.sql'

      # Fail if any destructive changes exist
      DROPS=$(jq '.summary.drops // 0' /tmp/migration-plan.json)
      RENAMES=$(jq '.summary.renames // 0' /tmp/migration-plan.json)
      if [ "$DROPS" -gt 0 ] || [ "$RENAMES" -gt 0 ]; then
        echo ""
        echo "❌ Destructive schema changes detected:"
        echo "   Drops: $DROPS, Renames: $RENAMES"
        echo "   Use the additive pattern instead (add new column → backfill → update code → later drop old)."
        exit 1
      fi

  # Drift detection — runs on schedule to alert if live DB diverges from schema.sql
  - name: Detect Schema Drift (Staging)
    env:
      DATABASE_URL: ${{ secrets.STAGING_DATABASE_URL }}
    run: |
      pgmold drift \
        -s sql:migrations/schema.sql \
        -d "$DATABASE_URL" \
        --json || { echo "⚠️ Schema drift detected between schema.sql and staging DB"; exit 1; }
```

This catches destructive changes at review time — before they reach staging or production. The PR author sees exactly what pgmold flagged and can either:
- Switch to the additive pattern (add new column, deprecate old one)
- Intentionally request a destructive override for the eventual cleanup deployment (manual process only)

##### Local Development Workflow

Local development uses pgmold for incremental changes and `docker compose` + raw SQL for nuclear resets:

```bash
# First time setup — spin up local Postgres via Docker Compose:
docker compose up -d postgres

# Bootstrap schema from scratch (drops everything, recreates from schema.sql):
./scripts/local-reset-db.sh
  # This script runs: DROP SCHEMA public CASCADE; CREATE SCHEMA public;
  # Then: psql "$LOCAL_DB_URL" < migrations/schema.sql
  # Fast, safe locally, guarantees exact match.

# On subsequent dev sessions — apply incremental changes only (non-destructive):
pgmold apply \
  -s sql:migrations/schema.sql \
  -d "postgres://localhost:5432/noms_dev"

# Preview what would change before applying:
pgmold plan \
  -s sql:migrations/schema.sql \
  -d "postgres://localhost:5432/noms_dev"
```

`local-reset-db.sh` is the nuclear option for development — it drops and recreates everything from `schema.sql`. It's fast, it's safe locally (your laptop), and it guarantees your dev database exactly matches the declared schema. For day-to-day work, `pgmold apply` adds new tables/columns/indexes without touching existing data.

##### Seed Data

Reference data (predefined tags, default settings) lives in `migrations/seed.sql` and is applied after schema migration:

```sql
-- seed.sql — idempotent inserts for reference data
INSERT INTO tags (name, category) VALUES
    ('Vegan', 'dietary'),
    ('GlutenFree', 'dietary'),
    ('Dinner', 'meal_type'),
    ('30min', 'difficulty')
ON CONFLICT (name) DO UPDATE SET category = EXCLUDED.category;
```

Every seed statement uses `INSERT ... ON CONFLICT DO UPDATE` so it's safe to run repeatedly. Applied as part of the deploy pipeline immediately after pgmold finishes schema migration:

```bash
# Deploy script sequence:
pgmold apply -s sql:migrations/schema.sql -d "$DATABASE_URL" && \
  psql "$DATABASE_URL" < migrations/seed.sql
```

##### Summary — Why This Approach Wins

| Concern | Traditional ORM Migrations | Our Declarative + pgmold Approach |
|---------|---------------------------|-----------------------------------|
| **Onboarding** | Read 47 incremental files to understand current state | Open `schema.sql` — done in 30 seconds |
| **Drift risk** | Migration file forgets a column → silent schema/code mismatch | `pgmold drift` detects divergence automatically; CI alerts on schedule |
| **Data safety** | A DROP in a migration file = gone forever (unless you notice in review) | pgmold blocks drops by default. Explicit `--allow-destructive` required with audit trail. Production mode blocks table drops entirely. |
| **Idempotency** | Each migration runs once; re-running breaks things | pgmold generates idempotent DDL (`IF NOT EXISTS`, transactional apply). Safe to retry on failure. |
| **Rollback** | Need explicit "down" migrations (often forgotten) | Nothing to roll back — old columns/tables just stay until cleaned up manually |
| **Lock hazards** | Developer must know PostgreSQL internals to avoid blocking | pgmold warns about lock hazards inline; uses concurrent index builds and online constraint validation by default |

#### 5. Secrets Management
Environment-specific secrets are managed directly within each Railway project's dashboard:
- **Staging Project:** Contains `STAGING_DATABASE_URL`, dummy OAuth client IDs/secrets pointing to staging callback URLs, and test R2 credentials.
- **Production Project:** Contains `PROD_DATABASE_URL`, live OAuth credentials with production redirect URIs, and production R2 keys.
- Railway automatically injects these into the container environment at build/run time — no GitHub Actions secrets or CLI tokens required for deployment.

### PostgreSQL & Infrastructure Considerations

Since we are relying on Railway's managed Postgres, there are three critical infrastructure considerations to address now so they don't become blockers later:

#### 1. Extension Compatibility (`pg_search` Caveat)
Railway's standard managed Postgres image includes most common extensions like `pg_trgm` and `pgvector`. However, **`pg_search` is a third-party extension** that often requires specific compilation flags or custom Docker images not available in Railway's default "one-click" database setup. 
- *Workaround:* If we absolutely need `pg_search`, we will deploy our own Postgres container on Railway instead of using their managed block. Alternatively, native PostgreSQL Full-Text Search (`tsvector` + `GIN indexes`) is incredibly fast and built-in, which might be sufficient for MVP keyword search without the extra infrastructure overhead.

#### 2. Connection Pooling (The "Too Many Connections" Problem)
Railway enforces strict concurrent connection limits on managed databases (often capping at 60 connections on lower tiers). Rust async runtimes are highly concurrent and can easily exhaust this limit if every HTTP request opens a fresh TCP socket to Postgres. 
- *Solution:* We must use a robust Rust-native connection pooler like **`deadpool-postgres`** in our Axum backend. It maintains a small, fixed set of persistent connections (e.g., 10 active + 20 idle) and safely routes all database queries through them, preventing "too many clients" crashes under load.

#### 3. Offsite Backup Strategy
Railway provides automatic daily snapshots for managed databases, but relying solely on a single provider's backup window is risky. 
- *Solution:* We will implement an offsite backup strategy using Cloudflare R2. A simple nightly GitHub Action (or Railway cron job) will run `pg_dump` against the production database and upload the compressed `.sql.gz` archive to our `noms-media/backups/` bucket. This gives us long-term, encrypted, point-in-time recovery independent of Railway's infrastructure.

---

## Data Model

### Core Entities

#### User
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(30) UNIQUE NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    avatar_url TEXT,
    bio TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### Recipe
```sql
CREATE TABLE recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(200) NOT NULL,
    description TEXT,
    prep_time INTERVAL,
    cook_time INTERVAL,
    servings INTEGER,
    difficulty_level VARCHAR(20), -- 'easy', 'medium', 'hard'
    is_public BOOLEAN DEFAULT TRUE,  -- Public-first default
    current_version_number INTEGER DEFAULT 1,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_recipes_owner ON recipes(owner_id);
CREATE INDEX idx_recipes_public ON recipes(is_public) WHERE is_public = TRUE;
CREATE INDEX idx_recipes_search ON recipes USING GIN(
    to_tsvector('english', title || ' ' COALESCE(description, ''))
);
```

#### RecipeVersion (Immutable Snapshots)
```sql
CREATE TABLE recipe_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    version_number INTEGER NOT NULL,
    title VARCHAR(200) NOT NULL,  -- Snapshot of title at this version
    description TEXT,
    ingredients JSONB NOT NULL,   -- Array of ingredient objects
    steps JSONB NOT NULL,         -- Array of step objects
    images JSONB,                 -- Array of image URLs/metadata
    metadata JSONB,               -- Flexible additional fields (servings, times, etc.)
    authored_by UUID NOT NULL REFERENCES users(id),
    change_summary TEXT,          -- Human-readable description of changes
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(recipe_id, version_number)
);

CREATE INDEX idx_versions_recipe ON recipe_versions(recipe_id, version_number DESC);

-- Ingredient structure example:
-- [
--   { "name": "flour", "quantity": 2, "unit": "cups", "notes": "" },
--   { "name": "salt", "quantity": 0.5, "unit": "tsp", "notes": "to taste" }
-- ]

-- Step structure example:
-- [
--   { "instruction": "Mix dry ingredients together", "order": 1, "image_url": null },
--   { "instruction": "Add wet ingredients gradually", "order": 2, "image_url": "/images/step-2.jpg" }
-- ]
```

#### ForkRelationship (DAG of Recipe Lineage)
```sql
CREATE TABLE fork_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_recipe_id UUID NOT NULL REFERENCES recipes(id),  -- The parent recipe in the lineage
    original_version_id UUID NOT NULL REFERENCES recipe_versions(id),  -- Specific version that was forked
    forked_recipe_id UUID NOT NULL REFERENCES recipes(id),   -- The new recipe created by forking
    forked_by UUID NOT NULL REFERENCES users(id),
    message TEXT,  -- Optional: why this recipe was forked / what changes are planned
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_forks_original ON fork_relationships(original_recipe_id);
CREATE INDEX idx_forks_result ON fork_relationships(forked_recipe_id);
```

**Graph traversal example:** To find all ancestors of a recipe (full lineage):
```sql
WITH RECURSIVE lineage AS (
    -- Start with the target recipe
    SELECT forked_recipe_id as recipe_id, original_recipe_id, 
           forked_by, created_at, 1 as depth
    FROM fork_relationships
    WHERE forked_recipe_id = :target_recipe_id

    UNION ALL

    -- Recursively find parents
    SELECT fr.original_recipe_id, fr.original_recipe_id,
           fr.forked_by, fr.created_at, l.depth + 1
    FROM fork_relationships fr
    INNER JOIN lineage l ON fr.forked_recipe_id = l.original_recipe_id
)
SELECT * FROM lineage ORDER BY depth DESC;
```

#### Collection (File System Model)
To support infinite nesting of folders efficiently, we use a **Materialized Path** strategy. This stores the entire ancestry chain in an array column (`path`), allowing us to query "all descendants of Folder X" instantly without expensive recursive joins.

```sql
CREATE TABLE collections (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id),
    parent_collection_id UUID REFERENCES collections(id) ON DELETE SET NULL, -- Allows for nesting
    
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_public BOOLEAN DEFAULT FALSE,
    
    -- Materialized Path: Stores the full hierarchy chain (e.g., [root_id, dinners_id, pasta_id])
    path UUID[] NOT NULL DEFAULT ARRAY[id], 
    
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_parent_name UNIQUE (owner_id, parent_collection_id, name)
);

-- GIN Index for lightning-fast "contains" queries on the path array
CREATE INDEX idx_collections_path ON collections USING GIN (path);

CREATE TABLE collection_recipes (
    collection_id UUID REFERENCES collections(id) ON DELETE CASCADE,
    recipe_id UUID REFERENCES recipes(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (collection_id, recipe_id)
);
```

**Performance Note:** With the `path` array, finding every single recipe inside a deeply nested folder is just one fast SQL query: `WHERE path @> ARRAY[ :folder_id ]`. This scales perfectly even if users create 50 levels of sub-folders (though UX best practices suggest keeping it around 3-5!).
```

#### Follow
```sql
CREATE TABLE follows (
    follower_id UUID NOT NULL REFERENCES users(id),
    following_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (follower_id, following_id),
    CHECK (follower_id != following_id)  -- Can't follow yourself
);

CREATE INDEX idx_follows_following ON follows(following_id);
```

#### Comment
```sql
CREATE TABLE comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    user_id UUID NOT NULL REFERENCES users(id),
    parent_comment_id UUID REFERENCES comments(id),  -- For threaded replies
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_comments_recipe ON comments(recipe_id, created_at DESC);
CREATE INDEX idx_comments_thread ON comments(parent_comment_id) WHERE parent_comment_id IS NOT NULL;

**Threading Strategy:** The `parent_comment_id` foreign key natively supports infinite nesting. Initially, we'll render flat top-level comments only (`parent_comment_id IS NULL`). When nested threads are added later, we have two rendering approaches to consider:
1. **Recursive CTE Backend:** Fetch the full tree structure server-side and return a deeply nested JSON payload. Simple for the frontend but can become slow on viral recipes with thousands of comments.
2. **Flat Query + Frontend Assembly (Recommended):** Fetch all comments for a recipe in one flat query (`ORDER BY created_at ASC`). The Dioxus frontend assembles them into a tree using the `parent_comment_id` and renders indentation dynamically. This is exactly how GitHub and Reddit handle massive comment sections efficiently, keeping database load minimal while allowing infinite UI nesting depth.

**UX Pattern Decision:** We'll adopt a **GitHub-style hybrid model**: 
- Allow true 1-level visual nesting (Reply → Reply) with clear indentation.
- Beyond 2 levels deep, linearize the thread visually but maintain logical grouping under the parent comment. This prevents the "infinite scroll to the right" problem seen on older forum software while preserving conversation context.
```

#### Like / Favorite
```sql
CREATE TABLE likes (
    user_id UUID NOT NULL REFERENCES users(id),
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, recipe_id)
);
```

#### Tags & Dietary Filters
Standardized tags power faceted search and dietary filtering without relying solely on freeform text parsing.
```sql
CREATE TABLE tags (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) UNIQUE NOT NULL,  -- e.g., 'Vegan', 'GlutenFree', 'Dinner'
    category VARCHAR(20) CHECK (category IN ('dietary', 'meal_type', 'difficulty', 'cuisine', 'custom')),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE recipe_tags (
    recipe_id UUID NOT NULL REFERENCES recipes(id),
    tag_id UUID NOT NULL REFERENCES tags(id),
    is_suggested BOOLEAN DEFAULT FALSE,  -- AI-suggested vs user-applied
    PRIMARY KEY (recipe_id, tag_id)
);

CREATE INDEX idx_recipe_tags_lookup ON recipe_tags(tag_id);
```

#### Pantry & Ingredient Discovery
A lightweight inventory system that pairs with ingredient indexing to answer "What can I cook right now?"
```sql
-- Predefined common ingredients for quick toggling and autocomplete
CREATE TABLE pantry_items_master (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) UNIQUE NOT NULL,  -- e.g., 'All-Purpose Flour', 'Olive Oil'
    category VARCHAR(50),              -- Produce, Dairy, Pantry Staples, etc.
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Leverages existing pg_trgm for instant ingredient search/autocomplete
CREATE INDEX idx_pantry_master_name_trgm ON pantry_items_master USING GIN (name gin_trgm_ops);

-- User's actual inventory state
CREATE TABLE user_pantry (
    user_id UUID NOT NULL REFERENCES users(id),
    item_id UUID NOT NULL REFERENCES pantry_items_master(id),
    quantity_on_hand DECIMAL(10, 2) DEFAULT 1.0,  -- Approximate baseline amount
    unit VARCHAR(20),
    expires_at DATE,
    last_updated TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (user_id, item_id)
);
```

#### Recipe Variations (Personal Branching)
Allows users to test private modifications under a single recipe before committing to a full public fork.
```sql
CREATE TABLE recipe_variations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id),
    base_recipe_id UUID NOT NULL REFERENCES recipes(id),  -- The original recipe being experimented on
    
    title VARCHAR(200) NOT NULL,  -- e.g., "Spicy Version", "Vegan Swap"
    ingredients JSONB NOT NULL,   -- Snapshot of modified ingredients
    steps JSONB NOT NULL,         -- Snapshot of modified steps
    notes TEXT,                   -- Private cook's notes for this variation
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_variations_base ON recipe_variations(base_recipe_id);
```

#### Meal Planner & Shopping Lists
Drag-and-drop planning that aggregates ingredients across multiple recipes into actionable lists.
```sql
CREATE TABLE meal_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id),
    plan_date DATE NOT NULL,
    meal_type VARCHAR(20) CHECK (meal_type IN ('breakfast', 'lunch', 'dinner', 'snack')),
    recipe_id UUID NOT NULL REFERENCES recipes(id),  -- Could be their own or a forked one
    notes TEXT,
    PRIMARY KEY (owner_id, plan_date, meal_type)
);

CREATE TABLE shopping_lists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES users(id),
    name VARCHAR(100) DEFAULT 'Weekly List',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_shopping_lists_owner ON shopping_lists(owner_id, is_active DESC);

CREATE TABLE shopping_list_items (
    list_id UUID NOT NULL REFERENCES shopping_lists(id) ON DELETE CASCADE,
    pantry_item_id UUID REFERENCES pantry_items_master(id),  -- Links to master if it's a known ingredient
    name VARCHAR(100) NOT NULL,                              -- Fallback for custom/untracked items
    quantity DECIMAL(10, 2),
    unit VARCHAR(20),
    is_purchased BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_list_items_lookup ON shopping_list_items(list_id);
```

#### Notifications
Tracks social interactions to drive in-app and email alerts.
```sql
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    type VARCHAR(30) NOT NULL CHECK (type IN ('recipe_forked', 'comment_received', 'user_followed', 'like_received')),
    actor_id UUID NOT NULL REFERENCES users(id),  -- Who triggered the notification
    target_recipe_id UUID REFERENCES recipes(id),
    message TEXT,                                 -- Pre-rendered summary for email/in-app display
    is_read BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_notifications_unread ON notifications(user_id, is_read, created_at DESC) WHERE is_read = FALSE;
```

### Relationships of Note
- **Recipe → one-to-many → RecipeVersion** (immutable snapshots enabling full history)
- **ForkRelationship creates a Directed Acyclic Graph (DAG)** of recipe lineage with recursive traversal support
- **User ↔ Follow ↔ User** (many-to-many directed relationship powering the social feed)

---

## UI/UX Design Principles

### Core Philosophy
1. **Cooking-first design:** Interfaces should be usable while cooking — large touch targets, high contrast, minimal scrolling during active use
2. **Progressive disclosure:** Simple interfaces for casual users, with advanced features (version history, fork graphs) accessible but not overwhelming
3. **Attribution by default:** Fork lineage and recipe origins are always visible but can be collapsed for cleaner reading

### Key Interaction Patterns
- **Recipe cards** as the primary unit of discovery — image-forward with key metadata overlay
- **Version timeline** presented as a scrollable history with diff highlighting (similar to GitHub's file changes view)
- **Fork graph visualization** showing recipe evolution as an interactive tree/DAG diagram
- **Inline editing** for quick tweaks without leaving the recipe context

### Accessibility Goals
- WCAG 2.1 AA compliance minimum
- Keyboard navigation for all interactive elements
- Screen reader optimized recipe structures (proper heading hierarchy, alt text for images)
- Color contrast ratios meeting accessibility standards for readability in kitchen environments

### Visual Design & Frontend Architecture

#### 1. Styling Strategy: Tailwind CSS + dioxus-components
Since Dioxus compiles to standard HTML/SVG on the web, we can leverage the entire modern CSS ecosystem without fighting a proprietary styling DSL. 
- **Tailwind CSS:** We will run a standard Tailwind CLI build step during development and production builds, outputting a single optimized `app.css`. This is then injected into Dioxus via the `asset!()` macro (`link { rel: "stylesheet", href: asset!("app.css") }`). This keeps our WASM bundle tiny (no runtime CSS-in-JS overhead) while giving us rapid UI development.
- **`dioxus-components` Library:** We will use the official Dioxus Labs component library ([github.com/DioxusLabs/components](https://github.com/DioxusLabs/components)) as our foundational UI kit. It is a Shadcn-inspired collection of ~40 accessible, customizable components (buttons, inputs, modals, dropdowns, cards, calendars, drag-and-drop lists, etc.) installable directly via the `dx add` CLI command. This gives us battle-tested accessibility and consistent visual language out of the box without fighting an immature ecosystem.
- **CSS Variables for Theming:** We'll define custom design tokens as CSS variables (`--color-primary`, `--spacing-md`) to override the library's defaults and inject our unique "GitHub for Recipes" brand identity. This supports light/dark mode toggling and future brand updates without touching component code.

#### 2. Component Architecture: Official Library + Custom Domain Components
Rather than building everything from scratch or relying on CSS-only plugins, we will adopt a **two-tier architecture**:
- **Infrastructure UI (`dioxus-components`):** Universal components like buttons, inputs, modals, alerts, dropdowns, badges, tabs, toasts, and tooltips are provided by the official library. These patterns rarely need customization and benefit from built-in accessibility (ARIA attributes, keyboard navigation) and responsive behavior. We add them incrementally via `dx add component_name` as we need them.
- **Domain-Specific Components (Custom):** Complex, unique features like the Recipe Card grid, Ingredient Scaling UI, Version Timeline diffs, Fork Graph visualization, and Meal Planner Calendar will be built as custom Dioxus components using our internal `rsx!` library. These require precise visual language and complex interactions where third-party kits provide zero value.
- **Type-Safe Composition:** All components—whether from the official library or custom-built—will have strictly typed Rust props (e.g., `<Button variant={ButtonVariant::Primary} disabled={true} />`). This eliminates entire classes of runtime bugs where missing or mistyped attributes break the UI.

#### 3. Available Official Components at a Glance
The `dioxus-components` library covers most standard UI patterns we'll need:
| Category | Available Components |
| :--- | :--- |
| **Inputs & Controls** | Button, Input, Textarea, Checkbox, Radio Group, Slider, Switch, Select, Combobox, Color Picker, Date Picker |
| **Overlays & Feedback** | Dialog/Modal, Alert Dialog, Popover, Tooltip, Toast, Progress, Skeleton (loading states) |
| **Navigation & Layout** | Navbar, Tabs, Accordion, Collapsible, Sidebar, Sheet (slide-out drawer), Pagination, Separator |
| **Data Display** | Card, Avatar, Badge, Table/Item, Hover Card, Scroll Area, Virtual List (high-performance long lists) |
| **Complex Interactions** | Drag & Drop List, Dropdown Menu, Context Menu, Menubar, Toggle Group, Toolbar |

This covers roughly 80% of our standard UI needs out of the box. We only build custom components for features that are unique to recipe management and social collaboration.

#### 4. Visual Aesthetic: Neumorphic + Glassmorphic + Subtle 3D
Rather than following the industry trend toward flat/minimalist interfaces, Noms will adopt a rich, tactile aesthetic that mirrors the physical nature of cooking and food preparation. We will blend three distinct visual styles strategically across the interface:

| Style | Visual Characteristics | Technical Implementation in Tailwind/Dioxus |
| :--- | :--- | :--- |
| **Glassmorphism** | Frosted translucency, soft blurred backgrounds, floating layered panels with thin luminous borders | `backdrop-filter: blur(12px)`, semi-transparent fills (`bg-white/20`), subtle white borders (`border border-white/30`). Best for sticky navbars, modals, and floating action panels. |
| **Neumorphism** | Soft extruded/inset shapes using balanced light/dual shadows against a monochromatic surface background. Clean "soft UI" aesthetic that feels modern and tactile without heavy textures or gradients | Dual directional `box-shadow` (light shadow top-left, dark shadow bottom-right) matching the base surface color but adjusted opacity/lightness. Pressed/active states invert to inset shadows (`inset 8px 8px 16px ..., -8px -8px 16px ...`). Best for buttons, ingredient toggles, scaling sliders, and interactive controls. |
| **Subtle 3D** | Perspective-aware hover tilts, floating parallax accents, depth separation between layered UI elements | CSS `transform-style: preserve-3d` with dynamic `rotateX/Y` on hover (driven by mouse position via Dioxus events), hardware-accelerated via `will-change: transform`. Best for recipe cards, fork graph nodes, and decorative hero elements. |

**Integration Pattern ("Glass/Neumo Shell Wrappers"):**
The official `dioxus-components` library defaults to a flat Shadcn aesthetic. Rather than fighting their internal CSS structure (which would break accessibility guarantees), we will wrap accessible flat primitives inside rich visual containers:
- The inner component retains its native keyboard navigation, ARIA attributes, and type-safe props.
- The outer shell applies our glass/neumorphic/3D styling via Tailwind utilities and CSS variables.
- This preserves 100% accessibility while allowing complete creative freedom over the final visual output.

**Strategic Placement Across Noms:**
| UI Area | Recommended Style | Rationale |
| :--- | :--- | :--- |
| **Sticky Top Navigation** | Glassmorphism | Blurs scrolling recipe images beneath it; feels modern and keeps content context visible while navigating |
| **Recipe Cards (Feed/Planner)** | 3D + Neumorphic Extrusion | Subtle hover tilt combined with soft dual shadows makes cards feel physically "pickable" like lightweight foam index cards |
| **Ingredient Inputs & Scaling Sliders** | Neumorphism | Soft extruded fields and inset pressed states provide clear tactile feedback that these are interactive controls you can push/pull while actively cooking |
| **Modals & Detail Drawers** | Glassmorphism | Frosted backdrop naturally dims background content without destroying readability or contrast ratios |
| **Fork Graph Nodes** | 3D Perspective + Neumo Depth | Slight `translateZ` separation between parent/child nodes paired with soft extrusion visually reinforces lineage depth and hierarchy without harsh drop shadows |
| **Version Timeline Diffs** | Neumorphic Inset Blocks | Green/red diff blocks use inverted inset shadows to appear "pressed into" the timeline surface, improving scanability while maintaining a cohesive soft aesthetic |

**Performance & Accessibility Safeguards:**
- `backdrop-filter: blur()` triggers compositor repaints. We will restrict it to sticky/fixed elements and large static panels. Rapidly scrolling content uses optimized semi-transparent fills instead.
- All 3D transforms are GPU-accelerated via CSS `transform` properties. Interactive tilt effects include a `will-change: transform` hint to prevent layout jank during hover states.
- Neumorphic shadow offsets, blur radii, and light/dark contrast ratios will be driven by CSS variables so we can globally toggle "High Contrast" or "Reduced Motion" modes for WCAG compliance without rewriting component styles. This is critical because neumorphism historically struggles with contrast requirements; our variable-driven approach ensures accessibility isn't sacrificed for aesthetics.

#### 5. Color System & Theme Architecture

**Perceived Temperature: Warm Earth Tones.** Noms should feel like stepping into a sunlit kitchen — warm, cozy, and inviting. The entire palette is built around baked-goods warmth (oatmeal surfaces, terracotta accents, basil greens) rather than sterile tech grays. This reinforces the emotional connection to cooking and food preparation.

##### Design Tokens (CSS Variables)

All colors are defined as CSS custom properties on `:root` (light mode default). Dark mode overrides via `[data-theme="dark"]`. Every component references tokens only — no hardcoded hex values in component styles. This means light/dark toggle is a single attribute swap with zero JavaScript overhead.

**Light Mode Palette:**

| Token | Value | Role |
|-------|-------|------|
| `--bg-base` | `#FAF7F2` | Page-level background — warm linen, the canvas everything sits on |
| `--surface` | `#F5F0E8` | Neumorphic element surface — buttons, cards, inputs. Slightly darker than bg so extrusion reads naturally |
| `--shadow-light` | `#FFFFFF` | Top-left neumorphic highlight (pure white) |
| `--shadow-dark` | `#DDD8CE` | Bottom-right neumorphic shadow (desaturated warm gray) |
| `--glass-fill` | `rgba(255, 255, 255, 0.20)` | Glassmorphic panel translucent fill |
| `--glass-border` | `rgba(255, 255, 255, 0.30)` | Thin luminous border on glass panels |
| `--accent` | `#D9735A` | Primary brand color — warm terracotta / dried tomato. CTAs, active states, links |
| `--accent-hover` | `#C4613F` | Deeper terracotta for hover/pressed CTA states |
| `--success` | `#5A9E6F` | Muted basil green — added diffs, checkmarks, completed states |
| `--warning` | `#D4923B` | Warm amber / turmeric — warnings, attention indicators |
| `--error` | `#C4504A` | Muted tomato red — removed diffs, destructive actions. Softer than pure red |
| `--text-primary` | `#2D2A26` | Headings, body text — warm near-black (not pure #000) |
| `--text-secondary` | `#7A756D` | Timestamps, metadata labels, placeholder text |
| `--text-tertiary` | `#A8A29A` | Disabled states, ghost icons, subtle dividers |

**Dark Mode Palette:**

| Token | Value | Role |
|-------|-------|------|
| `--bg-base` | `#1E1C18` | Deep warm charcoal — like a dark wood kitchen counter at night |
| `--surface` | `#242220` | Slightly lighter than bg for neumorphic extrusion on dark surfaces |
| `--shadow-light` | `#2E2B26` | Top-left highlight (warm gray, not white — white would blow out on dark) |
| `--shadow-dark` | `#141310` | Bottom-right shadow (near-black for grounding) |
| `--glass-fill` | `rgba(30, 28, 24, 0.50)` | Darker translucent fill for night-time glass panels |
| `--glass-border` | `rgba(255, 255, 255, 0.10)` | Subtler luminous border — less aggressive at night |
| `--accent` | `#E8896E` | Lighter terracotta for dark mode (maintains contrast on charcoal surfaces) |
| `--accent-hover` | `#F0A08C` | Even lighter hover variant to compensate for reduced emissive perception at night |
| `--success` | `#72B886` | Brighter basil green (dark surfaces absorb color, so we lift saturation) |
| `--warning` | `#E5A54E` | Lighter amber for dark mode readability |
| `--error` | `#D96B63` | Lighter tomato red — still muted, not neon |
| `--text-primary` | `#EDE8DF` | Warm off-white (not pure #FFF which creates harsh contrast on dark) |
| `--text-secondary` | `#9E978C` | Muted warm gray for secondary text |
| `--text-tertiary` | `#6B655D` | Darker muted gray for tertiary elements |

**Dark Mode Design Rationale:** The dark mode is NOT a simple invert. It's a separate emotional experience — like turning off the kitchen lights and cooking by warm ambient lamp light. Surfaces become deep charcoal (not pure black), shadows reverse their logic but use warmer tones, and accent colors lift in brightness to compensate for how dark backgrounds absorb perceived saturation. The terracotta brand identity remains intact throughout.

##### Animated Background Gradient

The page-level background is not static — it's a slow-shifting gradient that creates living warmth without being distracting. The cycle takes 30 seconds per full rotation, barely perceptible at a glance but creating an organic "breathing" quality to the interface.

```css
/* Light mode gradient */
:root {
    --bg-gradient-1: #FAF7F2;   /* warm oat — home base */
    --bg-gradient-2: #FFF3E8;   /* barely-there peach at 25% */
    --bg-gradient-3: #F0F4EC;   /* whisper of sage green at 50% */
    --bg-gradient-4: #FBF0E6;   /* soft apricot warmth at 75% */
}

/* Dark mode gradient — warm night tones */
[data-theme="dark"] {
    --bg-gradient-1: #1E1C18;   /* deep charcoal */
    --bg-gradient-2: #221F1A;   /* warmer amber undertone */
    --bg-gradient-3: #1A1D19;   /* subtle green shift (basil echo) */
    --bg-gradient-4: #201C17;   /* back toward warm brown */
}

body, .app-background {
    background: linear-gradient(
        135deg,
        var(--bg-gradient-1) 0%,
        var(--bg-gradient-2) 25%,
        var(--bg-gradient-3) 50%,
        var(--bg-gradient-4) 75%,
        var(--bg-gradient-1) 100%
    );
    background-size: 400% 400%;
    animation: bg-shift 30s ease infinite;
}

@keyframes bg-shift {
    0%   { background-position: 0% 50%; }
    50%  { background-position: 100% 50%; }
    100% { background-position: 0% 50%; }
}
```

**Why this works with glassmorphism:** The gradient is extremely muted — no single frame would look "colored" at a glance. But through frosted glass panels (`backdrop-filter: blur(12px)`), the subtle color variation bleeds through beautifully and shifts character as it animates. A sticky navbar or modal backdrop will gently change its undertone over the 30-second cycle, creating a living quality without any active animation on the panel itself.

**Reduced Motion consideration:** Users who prefer reduced motion get a static background (`animation: none`) set to `--bg-gradient-1` only. The gradient tokens still support this — we just don't animate between them.

##### Neumorphic Shadow Implementation

Neumorphism works by casting two shadows in opposite directions using slight brightness offsets of the surface color. On our warm palette, this reads as soft extruded foam or baked clay:

```css
/* Raised/extruded element (default state) */
.neumo-raised {
    background: var(--surface);
    border-radius: 12px;
    box-shadow:
        6px 6px 14px var(--shadow-dark),   /* bottom-right dark shadow */
       -6px -6px 14px var(--shadow-light); /* top-left highlight */
}

/* Pressed/inset element (active, focused, or selected state) */
.neumo-inset {
    background: var(--surface);
    border-radius: 12px;
    box-shadow:
        inset 6px 6px 14px var(--shadow-dark),
        inset -6px -6px 14px var(--shadow-light);
}

/* Larger card extrusion (recipe cards) */
.neumo-card {
    border-radius: 16px;
    box-shadow:
        8px 8px 20px var(--shadow-dark),
       -8px -8px 20px var(--shadow-light);
}
```

In light mode, `--shadow-light` is pure white and `--shadow-dark` is desaturated warm gray — the classic soft UI look. In dark mode, both shadows are warm grays (no pure black or pure white) which preserves the extrusion illusion without creating harsh contrast that would break the neumorphic effect entirely.

##### Tailwind CSS Configuration

We extend Tailwind's default theme to map our tokens into utility classes:

```js
// tailwind.config.js
module.exports = {
    theme: {
        extend: {
            colors: {
                surface: 'var(--surface)',
                bg: 'var(--bg-base)',
                accent: {
                    DEFAULT: 'var(--accent)',
                    hover: 'var(--accent-hover)',
                },
                success: 'var(--success)',
                warning: 'var(--warning)',
                error: 'var(--error)',
                text: {
                    primary: 'var(--text-primary)',
                    secondary: 'var(--text-secondary)',
                    tertiary: 'var(--text-tertiary)',
                },
            },
            boxShadow: {
                'neumo': '6px 6px 14px var(--shadow-dark), -6px -6px 14px var(--shadow-light)',
                'neumo-inset': 'inset 6px 6px 14px var(--shadow-dark), inset -6px -6px 14px var(--shadow-light)',
                'neumo-card': '8px 8px 20px var(--shadow-dark), -8px -8px 20px var(--shadow-light)',
            },
            backdropBlur: {
                'glass': '12px',
            },
        },
    },
};
```

This lets us write components like:
```rust
// Dioxus rsx! using Tailwind utilities
rsx! {
    div { class: "bg-surface shadow-neumo-card rounded-2xl p-6",
        h2 { class: "text-text-primary text-xl font-semibold", "Recipe Title" }
        button { class: "bg-accent hover:bg-accent-hover text-white px-4 py-2 rounded-lg shadow-neumo-inset",
            "Fork Recipe"
        }
    }
}
```

##### Theme Toggle Implementation Strategy

We design for dark mode parity from the start (all tokens defined, all components using variables), but we do not need to build the toggle UI immediately. The plumbing exists; the switch can be added whenever we want:

- **Storage:** User preference saved in a cookie (`theme=light|dark`) so SSR respects it on first load with no flash of wrong theme
- **System fallback:** If no explicit preference is set, respect `prefers-color-scheme` media query
- **Implementation:** A single `[data-theme="dark"]` attribute toggle on `<html>` or `<body>`. No JavaScript framework state needed — a lightweight Dioxus hook (`use_theme()`) reads/writes the cookie and flips the attribute. The CSS variable swap is instantaneous and GPU-composited (no layout thrash).

##### Layer Stack Summary

```
Layer 0: Animated gradient background (30s cycle, warm earth tones)
         └─ Light: oat → peach → sage → apricot → oat
            Dark: charcoal → amber-warm → green-shift → brown-warm → charcoal

Layer 1: Neumorphic surface elements (--surface with dual directional shadows)
         ├─ Recipe cards, ingredient inputs, buttons, controls, timeline blocks
         └─ Raised = extruded foam; Pressed = inset cavity

Layer 2: Glassmorphic overlays (translucent fill + backdrop blur + luminous border)
         ├─ Sticky navbar, modals, floating action panels, detail drawers
         └─ Gradient bleeds through frosted surfaces for living warmth

Layer 3: Terracotta accent (#D9735A light / #E8896E dark) for CTAs & active states
         └─ "Fork", "Save", "Publish" buttons — the only saturated color pop on screen

Layer 4: Semantic colors (success/warning/error) for diffs, badges, feedback
         └─ Always muted/natural tones — never neon or electric
```

#### 6. Key Visual Components & Elements
Based on our feature set, these are the primary UI elements we need to architect:

| Component | Visual Design & Interaction Pattern |
| :--- | :--- |
| **Recipe Card** | Image-forward design with a subtle gradient overlay at the bottom. Displays title, cook time, difficulty badge, and a "Forked X times" indicator. Hovering reveals quick actions (Like, Fork, Add to Planner). |
| **Ingredient Scaling UI** | A dynamic list where each row contains an ingredient name and a numeric quantity input. Changing the global "Servings: [ 4 ]" slider instantly recalculates all quantities via Rust math on the frontend. Includes a one-tap Metric ↔ Imperial toggle. |
| **Version Timeline** | Vertical scrollable history (similar to GitHub's file changes). Each version block highlights diffs in green (added) and red (removed) for ingredients/steps. A "Restore this version" button sits prominently at the top of each block. |
| **Fork Graph Visualization** | An SVG-based interactive DAG rendered directly in Dioxus. Nodes are circular recipe thumbnails; edges are bezier curves. Users can pan/zoom the canvas and click a node to drill down into that specific fork's details. |
| **Autocomplete Search Bar** | A sticky top-bar input with a debounced dropdown. Results are categorized into tabs (Recipes, Ingredients, Tags) to prevent visual clutter when typing short prefixes like "gar". |
| **Nested Comment Threads** | A linearized but visually indented conversation tree. Top-level comments have full avatars and timestamps; nested replies use smaller UI footprint with subtle left-border indentation to preserve vertical scrolling space. |
| **Meal Planner Calendar** | A 7-day horizontal scrollable grid (or standard monthly view). Users drag recipe cards from their library into specific day slots. The cell background highlights if the dragged recipe matches the meal type (`breakfast`, `dinner`). |

---

## Open Questions & Decisions

| # | Question | Options Considered | Decision | Rationale |
|---|----------|-------------------|----------|-----------|
| 1 | Default privacy model — are recipes public or private by default? | Public-first vs Private-first | **Public-first** ✅ | Drives network effects and community growth. Users can easily toggle to private if desired. Clear UI indicators for visibility status. |
| 2 | Recipe import quality — auto-parse only, or allow full manual entry too? | Auto-parse + manual edit vs Full manual entry always | TBD | Trade-off between convenience and control |
| 3 | Fork attribution — how visible is the fork chain? | Always visible breadcrumb vs Optional/collapsible | **Collapsible lineage** ✅ | Full graph tracking in backend, but UI shows concise "Forked from [User]'s [Recipe]" with expandable detail panel for full history |
| 4 | Tech stack choice for frontend | Next.js / Remix / SvelteKit / other | **Dioxus (Rust)** ✅ | Learning opportunity, shared types between FE/BE, performance benefits of WASM, strong type safety |
| 5 | Tech stack choice for backend | Node.js / Python / Go / Rust / other | **Rust (Axum)** ✅ | Performance, memory safety, shared codebase with frontend via Dioxus full-stack pattern |
| 6 | Database — SQL or NoSQL? | PostgreSQL vs MongoDB | **PostgreSQL** ✅ | Relational model fits naturally, JSONB flexibility, recursive CTEs for fork graphs, full-text search built-in |
| 7 | Image storage strategy | Self-hosted (S3-compatible) vs Third-party (Cloudinary, Imgix) | **Cloudflare R2** ✅ | S3-compatible API with zero egress fees. Perfect for frequently-served recipe images. Uses standard `aws-sdk-s3` client pointing at R2 endpoint. |
| 8 | Deployment platform | Vercel / Railway / Fly.io / AWS | **Railway + R2** ✅ | Excellent Rust/Docker support, managed Postgres instance, straightforward deployment workflow. Paired with Cloudflare R2 for cost-effective image storage. |

---

## Competitive Landscape

### Paprika
- **Strengths:** Mature product, excellent recipe import from URLs, clean UI, cross-platform sync, meal planning, shopping lists
- **Weaknesses:** No community/social features, no version history, no forking — purely personal tool, paid subscription model, closed ecosystem (recipes live in your account only)

### CopyMeThat
- **Strengths:** Recipe import, web-based, free tier available
- **Weaknesses:** Dated UI, limited social features, no versioning or forking

### Tasty / Yummly / AllRecipes
- **Strengths:** Massive recipe databases, discovery engine, brand recognition
- **Weaknesses:** Content is platform-owned (not user-generated in the same way), no personal management workflow, ad-heavy experiences

### ChefTap
- **Strengths:** Free, community recipes, simple interface
- **Weaknesses:** No version history, limited organization features, basic UX

### Where Noms Fits
Noms occupies a unique intersection: **personal recipe management** (Paprika's strength) + **community collaboration with forking and versioning** (no one does this well). The fork/version model is the defensible moat — it creates network effects as recipes get remixed and improved across the community.

### Dioxus Ecosystem Considerations
- **Current state:** Actively developed, growing rapidly, but smaller ecosystem than React/Next.js
- **Advantages for Noms:** Single language (Rust) across full stack, compile-time safety, excellent performance characteristics
- **Potential challenges during development:** Fewer third-party UI component libraries, potentially longer debugging cycles while learning the framework, smaller community for troubleshooting
- **Mitigation strategy:** Use Dioxus core components and build custom UI primitives. Leverage Rust's excellent documentation and growing tutorial ecosystem. Consider contributing back to the community as we build.
