# How fspec Controls the Browser

This document explains how fspec browses the web compared to other AI assistants.

---

## The Simple Difference

**Traditional AI assistants** need a "middleman" to talk to the browser.

**fspec** talks to the browser directly.

---

## Side-by-Side Comparison

```mermaid
flowchart LR
    subgraph Traditional["Traditional Approach"]
        direction TB
        A1["🤖 AI Assistant"]
        A2["📡 Middleman Server"]
        A3["🔧 Automation Library"]
        A4["🌐 Browser"]
        
        A1 --> A2 --> A3 --> A4
    end
    
    subgraph fspec["fspec Approach"]
        direction TB
        B1["🤖 fspec"]
        B2["🌐 Browser"]
        
        B1 --> B2
    end
```

---

## Traditional Approach (Multiple Steps)

When a typical AI assistant wants to search the web or take a screenshot:

```mermaid
flowchart TB
    User["👤 You ask a question"]
    AI["🤖 AI Assistant"]
    Bridge["📡 Middleman Server<br/>(separate program)"]
    Library["🔧 Automation Tool<br/>(Playwright)"]
    Browser["🌐 Chrome Browser"]
    
    User --> AI
    AI -->|"1️⃣ Send request"| Bridge
    Bridge -->|"2️⃣ Translate"| Library
    Library -->|"3️⃣ Control"| Browser
    Browser -->|"4️⃣ Results back"| Library
    Library -->|"5️⃣ Translate back"| Bridge
    Bridge -->|"6️⃣ Send response"| AI
```

**Problems with this:**
- 🐌 Slower (6 steps instead of 2)
- 🔧 More things that can break
- 📦 Extra software to install and maintain

---

## fspec Approach (Direct Connection)

When fspec wants to search the web or take a screenshot:

```mermaid
flowchart TB
    User["👤 You ask a question"]
    fspec["🤖 fspec"]
    Browser["🌐 Chrome Browser"]
    
    User --> fspec
    fspec -->|"1️⃣ Direct control"| Browser
    Browser -->|"2️⃣ Results back"| fspec
```

**Benefits:**
- ⚡ Faster (direct connection)
- 🎯 Simpler (fewer moving parts)
- 📦 Nothing extra to install

---

## What Happens When You Ask fspec to Search

```mermaid
flowchart LR
    Ask["💬 You: Search for X"]
    Tool["🔧 WebSearch Tool"]
    Browser["🌐 Opens Chrome"]
    Search["🔍 Goes to DuckDuckGo"]
    Results["📋 Gets results"]
    Answer["💬 Shows you answer"]
    
    Ask --> Tool --> Browser --> Search --> Results --> Answer
```

---

## What Happens When You Ask for a Screenshot

```mermaid
flowchart LR
    Ask["💬 You: Screenshot this page"]
    Tool["🔧 Screenshot Tool"]
    Browser["🌐 Opens Chrome"]
    Load["📄 Loads the page"]
    Capture["📸 Takes picture"]
    Save["💾 Saves image"]
    Show["🖼️ Shows you"]
    
    Ask --> Tool --> Browser --> Load --> Capture --> Save --> Show
```

---

## Why Does This Matter?

| | Traditional | fspec |
|---|---|---|
| **Speed** | Slower | ⚡ Faster |
| **Setup** | Install extra servers | ✅ Works out of the box |
| **Reliability** | More can go wrong | ✅ Simpler = fewer problems |
| **Memory** | Multiple programs running | ✅ Single program |

> **In short:** fewer moving parts means fspec is faster, more reliable, and easier to set up than the traditional middleman approach.

---

## Technical Details (For Developers)

<details>
<summary>Click to expand technical architecture</summary>

### Component Stack

```mermaid
flowchart TB
    Facades["Provider Facades<br/>(Claude, Gemini)"]
    Registry["Tool Registry"]
    WebSearch["WebSearch Tool"]
    SearchEngine["Search Engine<br/>(DuckDuckGo)"]
    PageFetcher["Page Fetcher"]
    Screenshot["Screenshot"]
    ChromeBrowser["Chrome Browser Wrapper"]
    CDP["Chrome DevTools Protocol"]
    Chrome["Chrome Binary"]
    
    Facades --> Registry --> WebSearch
    WebSearch --> SearchEngine & PageFetcher & Screenshot
    SearchEngine & PageFetcher & Screenshot --> ChromeBrowser
    ChromeBrowser --> CDP --> Chrome
```

### Browser Connection Modes

```mermaid
flowchart TB
    Config["Configuration"]
    
    Check1{"Remote URL set?"}
    Check2{"Custom path set?"}
    
    Remote["Connect to existing Chrome"]
    Custom["Launch specified Chrome"]
    Auto["Auto-detect Chrome"]
    
    Config --> Check1
    Check1 -->|Yes| Remote
    Check1 -->|No| Check2
    Check2 -->|Yes| Custom
    Check2 -->|No| Auto
```

### Environment Variables

| Variable | What it does |
|----------|-------------|
| `CODELET_CHROME_WS_URL` | Connect to an already-running Chrome |
| `CODELET_CHROME_PATH` | Use a specific Chrome installation |
| `CODELET_CHROME_HEADLESS` | Show/hide the browser window |
| `CODELET_CHROME_TIMEOUT` | How long before closing idle browser |

### Related Code Files

- `codelet/tools/src/chrome_browser.rs` - Browser control
- `codelet/tools/src/web_search.rs` - Search and screenshot
- `codelet/tools/src/search_engine.rs` - DuckDuckGo integration
- `codelet/tools/src/page_fetcher.rs` - Page content extraction

</details>
