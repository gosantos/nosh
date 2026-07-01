use chrono::{NaiveDateTime, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: u64,
    description: String,
    done: bool,
    #[serde(default)]
    archived: bool,
    created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
struct Note {
    id: u64,
    title: String,
    content: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

fn seed_id(base_ts: i64, seq: u64) -> u64 {
    (base_ts as u64) << 10 | (seq % 1024)
}

fn main() {
    let notes_path = home().join(".tui-todo-notes.json");
    let todos_path = home().join(".tui-todo.json");

    // ── NOTES ──────────────────────────────────────────────────────
    let notes_data: Vec<(&str, &str, &str)> = vec![
        (
            "Project Architecture Overview",
            "2025-06-01 09:00:00",
            "# Project Architecture Overview\n\nThis document describes the high-level architecture of our microservices platform.\n\n## System Components\n\n- **API Gateway**: Routes incoming requests to appropriate services\n- **Auth Service**: Handles authentication and authorization\n- **User Service**: Manages user profiles and preferences\n- **Billing Service**: Processes payments and invoices\n\n## Data Flow\n\n```\nClient → API Gateway → Auth (JWT validation) → Service → Database\n```\n\n## Key Decisions\n\n1. We chose **PostgreSQL** for transactional data because of its reliability and JSON support.\n2. **Redis** is used for caching session data and rate limiting.\n3. Messages between services go through **RabbitMQ** for guaranteed delivery.\n\n## Scaling Strategy\n\nHorizontal scaling at the service level. Each service is stateless and can be replicated independently.",
        ),
        (
            "Meeting Notes - Sprint Planning",
            "2025-06-03 14:30:00",
            "# Sprint Planning — Week of June 3\n\n**Attendees**: Alice, Bob, Charlie, Diana\n\n## Goals for this sprint\n\n- [ ] Finish user dashboard redesign\n- [ ] Implement payment webhook handler\n- [ ] Write integration tests for auth flow\n- [ ] Deploy staging environment\n\n## Discussion points\n\n### Dashboard redesign\nAlice will lead the frontend work. We agreed on a card-based layout with customizable widgets.\n\n### Payment webhooks\nBob pointed out that Stripe sends events asynchronously. We need idempotency keys.\n\n### Testing\nCharlie will set up the test harness. Target: 80% coverage on new code.\n\n## Action items\n\n| Person | Task | Deadline |\n|--------|------|----------|\n| Alice | Dashboard mockup | Wed EOD |\n| Bob | Webhook endpoint | Thu EOD |\n| Charlie | Test setup | Fri EOD |\n| Diana | Deploy pipeline | Fri EOD |",
        ),
        (
            "Rust Ownership Rules Cheatsheet",
            "2025-06-05 11:15:00",
            "# Rust Ownership Rules\n\nQuick reference for Rust's ownership model.\n\n## Three Rules\n\n1. Each value has exactly **one owner** at a time\n2. When the owner goes out of scope, the value is dropped\n3. You may have **either** one mutable reference or any number of immutable references\n\n## Borrowing\n\n```rust\nfn read(s: &String) {\n    println!(\"{s}\");\n}\n\nfn write(s: &mut String) {\n    s.push_str(\"!\");\n}\n```\n\n## Common Lifetime Patterns\n\n```rust\nfn longest<'a>(x: &'a str, y: &'a str) -> &'a str {\n    if x.len() > y.len() { x } else { y }\n}\n```\n\n## Smart Pointers\n\n- `Box<T>` — heap allocation\n- `Rc<T>` — reference counting (single-threaded)\n- `Arc<T>` — atomic reference counting (multi-threaded)\n- `RefCell<T>` — interior mutability\n\n*Remember: `Rc<RefCell<T>>` is your escape hatch.*",
        ),
        (
            "Sourdough Starter Recipe",
            "2025-06-07 20:00:00",
            "# Sourdough Starter\n\nMaking a sourdough starter from scratch takes about 7-10 days.\n\n## Ingredients\n\n- 500g whole wheat flour (or rye)\n- 500g all-purpose flour\n- Filtered water (non-chlorinated)\n\n## Day 1\nMix 100g whole wheat flour + 100g water in a jar. Cover loosely. Leave at room temp (21-24°C).\n\n## Days 2-6\nEach day, discard half and feed with 100g flour + 100g water.\n\n**Signs of a healthy starter:**\n- Bubbles form 4-8 hours after feeding\n- Sweet, yeasty smell (not acetone or nail polish)\n- Doubles in volume within 6 hours\n\n## Maintenance\n- Keep in fridge if feeding less than once a week\n- If baking daily, keep on counter and feed once a day\n- Always use room temperature water\n\n> \"A good starter is like a pet — feed it regularly and it will reward you.\"",
        ),
        (
            "API Design Guidelines",
            "2025-06-08 10:30:00",
            "# API Design Guidelines\n\n## URL Structure\n\n```\nGET    /api/v1/users          # List users\nPOST   /api/v1/users          # Create user\nGET    /api/v1/users/:id      # Get user by ID\nPATCH  /api/v1/users/:id      # Update user\nDELETE /api/v1/users/:id      # Delete user\n```\n\n## Pagination\n\nAlways paginate list endpoints. Use cursor-based pagination for consistency.\n\n```json\n{\n  \"data\": [...],\n  \"next_cursor\": \"eyJpZCI6IDU2fQ==\",\n  \"has_more\": true\n}\n```\n\n## Error Responses\n\n```json\n{\n  \"error\": {\n    \"code\": \"VALIDATION_ERROR\",\n    \"message\": \"Email is required\",\n    \"details\": {\n      \"field\": \"email\",\n      \"reason\": \"missing\"\n    }\n  }\n}\n```\n\n## Rate Limiting\n\nReturn these headers on every response:\n\n- `X-RateLimit-Limit`\n- `X-RateLimit-Remaining`\n- `X-RateLimit-Reset`\n\n## Versioning\n\nVersion via URL prefix (`/api/v1/`). Support each version for at least 6 months.",
        ),
        (
            "Grocery List & Meal Plan",
            "2025-06-09 18:00:00",
            "# Grocery List\n\n## Produce\n\n- Bananas (bunch)\n- Avocados (3)\n- Baby spinach\n- Cherry tomatoes\n- Red bell peppers\n- Garlic (head)\n- Lemons (2)\n- Cilantro\n- Sweet potatoes (3)\n\n## Proteins\n\n- Chicken thighs (boneless, 1lb)\n- Ground beef (80/20, 1lb)\n- Eggs (dozen)\n- Greek yogurt\n- Firm tofu\n\n## Pantry\n\n- Olive oil\n- Soy sauce\n- Canned black beans\n- Quinoa\n- Brown rice\n- Almond butter\n\n## Meal Plan\n\n| Day | Breakfast | Lunch | Dinner |\n|-----|-----------|-------|--------|\n| Mon | Oatmeal | Salad | Stir fry |\n| Tue | Smoothie | Wrap | Pasta |\n| Wed | Eggs | Bowl | Tacos |\n| Thu | Yogurt | Sandwich | Curry |\n| Fri | Pancakes | Leftovers | Pizza |",
        ),
        (
            "Refactoring Notes - Legacy Code",
            "2025-06-10 16:45:00",
            "# Legacy Code Refactoring Plan\n\n## Current Problems\n\nThe `OrderProcessor` class has grown to 2,500 lines. It violates:\n\n- **SRP**: Handles validation, payment, inventory, shipping, and notifications\n- **OCP**: Every new payment method requires modifying the class\n- **DIP**: Direct dependency on Stripe SDK, FedEx API, and SMTP client\n\n## Refactoring Strategy\n\n### Phase 1: Extract services (estimated 3 days)\n\n```\nOrderProcessor\n├── OrderValidator\n├── PaymentService (interface)\n│   ├── StripePayment\n│   └── PayPalPayment\n├── InventoryService\n├── ShippingService (interface)\n│   ├── FedExShipping\n│   └── UPSService\n└── NotificationService\n```\n\n### Phase 2: Introduce repositories (2 days)\n\nMove all database queries into repository classes.\n\n### Phase 3: Add tests (2 days)\n\nWrite unit tests for each extracted service. Mock external dependencies.\n\n## Risk Assessment\n\n- **High**: Payment processing — careful with the migration\n- **Medium**: Shipping label generation — format changes could break customers\n- **Low**: Email notifications — easy to swap\n\n*Strategy: Roll out one service at a time behind a feature flag.*",
        ),
        (
            "Travel Journal - Japan Trip",
            "2025-06-12 22:00:00",
            "# Japan Trip 2025\n\n## Day 1 — Tokyo (Shibuya)\n\nArrived at Narita around 3 PM. Took the Narita Express to Shinjuku. Checked into a tiny but efficient hotel room — 12 sq meters, immaculate.\n\nWent to Shibuya Crossing at night. The scale of it is unreal. Thousands of people crossing from every direction. Grabbed ramen at Ichiran — solo booths where nobody talks to you. Perfect.\n\n## Day 2 — Asakusa & Akihabara\n\n**Senso-ji Temple** early morning before the crowds. The giant red lantern at Kaminarimon gate is iconic. Got some matcha ice cream.\n\nSpent the afternoon in Akihabara. Went to a retro game shop and found a Famicom in good condition for ¥8,000. Too good to pass up.\n\n## Day 3 — Kyoto\n\nShinkansen from Tokyo to Kyoto — 2.5 hours, flawless. Bamboo Grove at dawn (go at 6 AM, trust me).\n\nFushimi Inari Taisha — hiked the full trail. Took about 2 hours. The thousands of vermillion torii gates create this surreal tunnel effect.\n\n## Tips\n\n- Get a SUICA card at the airport\n- Google Maps works perfectly for transit\n- Most ATMs in 7-Eleven accept foreign cards\n- Learn \"sumimasen\" (excuse me) and \"arigato\" (thank you)",
        ),
        (
            "Vim Shortcuts Reference",
            "2025-06-14 08:00:00",
            "# Vim Shortcuts\n\n## Navigation\n\n| Key | Action |\n|-----|--------|\n| `h/j/k/l` | Left/Down/Up/Right |\n| `w` / `b` | Next/previous word |\n| `0` / `$` | Start/end of line |\n| `gg` / `G` | First/last line |\n| `Ctrl+d` / `Ctrl+u` | Page down/up |\n| `{` / `}` | Paragraph up/down |\n\n## Editing\n\n| Key | Action |\n|-----|--------|\n| `x` | Delete character |\n| `dd` | Delete line |\n| `yy` | Yank (copy) line |\n| `p` / `P` | Paste after/before |\n| `u` | Undo |\n| `Ctrl+r` | Redo |\n| `.` | Repeat last change |\n\n## Visual Mode\n\n- `v` — character-wise visual\n- `V` — line-wise visual\n- `Ctrl+v` — block-wise visual\n\n## Search\n\n- `/pattern` — search forward\n- `?pattern` — search backward\n- `n` / `N` — next/previous match\n- `*` / `#` — search word under cursor forward/backward\n\n*Pro tip: `:set relativenumber` changes your life.*",
        ),
        (
            "Book Notes - Designing Data-Intensive Applications",
            "2025-06-15 21:30:00",
            "# Designing Data-Intensive Applications\n\nBy Martin Kleppmann\n\n## Chapter 1: Reliable, Scalable, and Maintainable Apps\n\n### Reliability\n- System should work correctly even when things go wrong\n- Faults vs failures: faults are expected, failures are when the system stops serving\n- Hardware faults (disk failure, power outage), software bugs, human errors\n\n### Scalability\n- If growth is predictable, vertical scaling is cheaper\n- If unpredictable, horizontal scaling with load balancing\n- Benchmark with realistic workloads, not synthetic\n\n### Maintainability\n- **Operability**: Make it easy for ops teams to run\n- **Simplicity**: Reduce complexity through abstraction\n- **Evolvability**: Make change easy\n\n## Chapter 2: Data Models and Query Languages\n\n- Relational vs NoSQL is not a real dichotomy — use the right tool\n- Document databases: better for hierarchical data\n- Graph databases: complex many-to-many relationships\n\n> \"Most applications are not built on a single data model — they layer multiple models.\"\n\n## Key Takeaways\n\n1. Always plan for faults\n2. Benchmark before optimizing\n3. Prefer composition over inheritance in data models",
        ),
        (
            "Home Network Setup",
            "2025-06-16 14:00:00",
            "# Home Network Configuration\n\n## Hardware\n\n- **Router**: Ubiquiti EdgeRouter X\n- **AP**: TP-Link EAP225 (x2, wired backhaul)\n- **Switch**: Netgear GS108 (8-port gigabit)\n- **Server**: Raspberry Pi 4 (8GB) running Pi-hole + Home Assistant\n\n## VLAN Layout\n\n| VLAN | Subnet | Purpose |\n|------|--------|---------|\n| 10 | 10.0.10.0/24 | Trusted (computers, phones) |\n| 20 | 10.0.20.0/24 | IoT (lights, plugs, sensors) |\n| 30 | 10.0.30.0/24 | Guest WiFi |\n| 40 | 10.0.40.0/24 | Servers (Pi, NAS) |\n\n## DNS\n\nPi-hole blocks ads at the network level. Upstream DNS: Cloudflare 1.1.1.1 and Quad9 9.9.9.9.\n\n## Services\n\n| Service | Port | VLAN |\n|---------|------|------|\n| Pi-hole | 53/80 | 40 |\n| Home Assistant | 8123 | 40 |\n| NAS SMB | 445 | 40 |\n| Jellyfin | 8096 | 40 |\n\n## WiFi\n\n- 2.4 GHz for IoT devices (better range)\n- 5 GHz for computers/phones (higher throughput)\n- Guest network isolated from LAN",
        ),
        (
            "PostgreSQL Query Optimization",
            "2025-06-17 10:00:00",
            "# PostgreSQL Query Optimization Notes\n\n## 1. Use EXPLAIN ANALYZE\n\n```sql\nEXPLAIN ANALYZE SELECT * FROM orders WHERE status = 'pending';\n```\n\nLook for:\n- Sequential scans on large tables\n- High `rows` vs `actual rows` estimates\n- Sort operations without indexes\n\n## 2. Indexing Strategies\n\n### B-tree indexes\nDefault index type. Good for equality and range queries.\n\n```sql\nCREATE INDEX idx_orders_status ON orders(status);\nCREATE INDEX idx_orders_created ON orders(created_at DESC);\n```\n\n### Composite indexes\nColumn order matters: put equality columns first, range columns last.\n\n```sql\nCREATE INDEX idx_orders_user_status ON orders(user_id, status);\n```\n\n### Partial indexes\nIndex only the rows you query.\n\n```sql\nCREATE INDEX idx_orders_pending ON orders(status) WHERE status = 'pending';\n```\n\n## 3. Common Pitfalls\n\n- **N+1 queries**: Use JOIN or batch loading\n- **Missing indexes on foreign keys**: Every FK should be indexed\n- **SELECT ***: Only fetch columns you need\n- **OR conditions**: Can defeat index usage; try UNION instead\n\n## 4. Configuration Tuning\n\n```ini\nshared_buffers = 25% of RAM\neffective_cache_size = 50% of RAM\nwork_mem = 32MB (per operation)\nmaintenance_work_mem = 256MB\n```",
        ),
        (
            "Fitness Routine - June",
            "2025-06-18 07:30:00",
            "# June Fitness Log\n\n## Weekly Schedule\n\n| Day | Morning | Evening |\n|-----|---------|---------|\n| Mon | Run (5K) | Rest |\n| Tue | Rest | Upper body |\n| Wed | Run (intervals) | Rest |\n| Thu | Rest | Lower body |\n| Fri | Run (5K) | Rest |\n| Sat | Rest | Full body |\n| Sun | Long run (10K) | Stretch |\n\n## Current Maxes\n\n- Bench: 185 lbs\n- Squat: 225 lbs\n- Deadlift: 275 lbs\n- 5K: 22:30\n\n## Nutrition\n\n- Protein: 180g/day\n- Calories: ~2,800 maintenance\n- Water: 3L minimum\n\n## Progress Tracking\n\nWeight this month: 172 → 175 lbs (lean bulking). Waist unchanged at 32 inches.\n\n*Goal by August: 185 bench, 245 squat, 315 deadlift.*",
        ),
        (
            "Docker Compose Patterns",
            "2025-06-19 15:00:00",
            "# Docker Compose Patterns\n\n## Development Setup\n\n```yaml\nversion: '3.8'\nservices:\n  app:\n    build: .\n    volumes:\n      - .:/app\n    ports:\n      - \"3000:3000\"\n    environment:\n      - DATABASE_URL=postgres://user:pass@db:5432/app\n    depends_on:\n      - db\n\n  db:\n    image: postgres:16\n    volumes:\n      - pgdata:/var/lib/postgresql/data\n    environment:\n      POSTGRES_PASSWORD: pass\n\nvolumes:\n  pgdata:\n```\n\n## Multi-stage Build\n\n```dockerfile\nFROM rust:1.77 AS builder\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n\nFROM debian:bookworm-slim\nCOPY --from=builder /app/target/release/app /usr/local/bin/\nCMD [\"app\"]\n```\n\n## Health Checks\n\n```yaml\nhealthcheck:\n  test: [\"CMD\", \"curl\", \"-f\", \"http://localhost:8080/health\"]\n  interval: 30s\n  timeout: 10s\n  retries: 3\n  start_period: 40s\n```\n\n## Tips\n\n- Use `.env` files for environment-specific config\n- Pin image versions (never use `latest`)\n- Use `restart: unless-stopped` for production services\n- Set memory limits: `mem_limit: 512m`",
        ),
        (
            "Weekend Project - CLI Pomodoro Timer",
            "2025-06-20 13:00:00",
            "# CLI Pomodoro Timer in Rust\n\nBuilding a minimal pomodoro timer as a weekend project.\n\n## Requirements\n\n- 25-minute focus sessions\n- 5-minute breaks\n- Notification when timer completes\n- Pause/resume support\n- Simple TUI or CLI interface\n\n## Initial Design\n\n```rust\nstruct Pomodoro {\n    focus_duration: Duration,\n    break_duration: Duration,\n    state: State,\n    remaining: Duration,\n}\n\nenum State {\n    Focus,\n    Break,\n    Paused,\n    Idle,\n}\n```\n\n## Implementation Plan\n\n1. Parse CLI args with `clap`\n2. Main loop with `std::thread::sleep` and `Instant::now`\n3. Use `crossterm` for terminal bell\n4. Cross-platform notifications via `notify-rust`\n\n## Progress\n\n- [x] Basic timer logic\n- [x] CLI argument parsing\n- [ ] Terminal output with ratatui\n- [ ] Pause/resume\n- [ ] Desktop notifications\n- [ ] Session history log\n\n## Notes\n\nStarted on Friday evening. Basic timer works. Need to make the TUI display nicer — a progress bar would be cool.\n\n*Update: Got the progress bar working with ratatui gauges. Looks great.*",
        ),
        (
            "Investment Portfolio Review",
            "2025-06-21 19:00:00",
            "# Portfolio Review — Q2 2025\n\n## Asset Allocation\n\n| Asset | Target | Current |\n|-------|--------|--------|\n| US Stocks (VTI) | 50% | 52% |\n| International (VXUS) | 20% | 18% |\n| Bonds (BND) | 20% | 20% |\n| REIT (VNQ) | 5% | 4% |\n| Cash | 5% | 6% |\n\n## Performance\n\n- YTD return: +7.2%\n- Benchmark (S&P 500): +8.1%\n- Underperformance due to international exposure (EUR weakened)\n\n## Actions\n\n1. Rebalance: sell some VTI, buy VXUS and VNQ\n2. Increase 401(k) contribution from 10% to 12%\n3. Open a Roth IRA before year end\n4. Set up automatic weekly investments\n\n## Notes\n\nMarket volatility expected in H2 due to election year. Stay the course — time in the market beats timing the market.\n\n*Next review: September 2025*",
        ),
        (
            "Indoor Plants Care Guide",
            "2025-06-22 11:00:00",
            "# Plant Care Guide\n\n## Current Collection\n\n### Monstera Deliciosa (Swiss Cheese Plant)\n- **Light**: Bright indirect\n- **Water**: Every 7-10 days, let soil dry between\n- **Notes**: Getting huge — needs a moss pole soon\n\n### Pothos (Golden)\n- **Light**: Low to bright indirect\n- **Water**: Every 10-14 days\n- **Notes**: Propagated 3 cuttings, all rooting in water\n\n### Snake Plant (Sansevieria)\n- **Light**: Low to bright indirect\n- **Water**: Every 3-4 weeks (very drought tolerant)\n- **Notes**: Almost impossible to kill. Perfect for beginners.\n\n### Fiddle Leaf Fig\n- **Light**: Bright indirect, direct morning sun ok\n- **Water**: Every 5-7 days\n- **Notes**: Picky. Dropped leaves when moved. Needs consistent watering.\n\n### ZZ Plant\n- **Light**: Low to bright indirect\n- **Water**: Every 3-4 weeks\n- **Notes**: Thriving despite neglect. New shoots coming up.\n\n## Fertilizing Schedule\n\n| Season | Frequency |\n|--------|-----------|\n| Spring | Every 2 weeks |\n| Summer | Every 2 weeks |\n| Fall | Every 4 weeks |\n| Winter | Stop |\n\n*Use half-strength liquid fertilizer.*",
        ),
        (
            "Kubernetes Cheatsheet",
            "2025-06-23 09:30:00",
            "# Kubernetes Commands\n\n## Context & Clusters\n\n```bash\nkubectl config get-contexts\nkubectl config use-context production\nkubectl config current-context\n```\n\n## Pod Operations\n\n```bash\nkubectl get pods\nkubectl get pods -w                    # Watch mode\nkubectl describe pod <name>\nkubectl logs -f <pod>\nkubectl exec -it <pod> -- /bin/sh\n```\n\n## Deployments\n\n```bash\nkubectl get deployments\nkubectl rollout status deploy/<name>\nkubectl rollout undo deploy/<name>     # Rollback to previous\nkubectl scale deploy/<name> --replicas=5\n```\n\n## Services\n\n```bash\nkubectl get svc\nkubectl port-forward svc/<name> 8080:80\n```\n\n## Debugging\n\n```bash\nkubectl get events --sort-by='.lastTimestamp'\nkubectl top pods                      # Resource usage\nkubectl run tmp --image=busybox -it --rm -- /bin/sh\n```\n\n## Useful Aliases\n\n```bash\nalias k='kubectl'\nalias kgp='kubectl get pods'\nalias kdp='kubectl describe pod'\nalias kex='kubectl exec -it'\nalias kga='kubectl get all'\n```\n\n## YAML Shortcuts\n\n```yaml\n# Quick pod\napiVersion: v1\nkind: Pod\nmetadata:\n  name: nginx\nspec:\n  containers:\n  - name: nginx\n    image: nginx:alpine\n```",
        ),
        (
            "House Renovation Tracker",
            "2025-06-24 20:00:00",
            "# Renovation Project\n\n## Kitchen\n\n| Item | Status | Contractor | Cost |\n|------|--------|------------|------|\n| Demolition | ✅ Done | — | $800 |\n| Electrical | ✅ Done | Mike | $2,400 |\n| Plumbing | 🔄 In progress | Joe | $1,800 |\n| Cabinets | 📦 Ordered | IKEA | $4,200 |\n| Countertop | 📦 Ordered | GraniteCo | $2,800 |\n| Backsplash | ⏳ Pending | — | $600 |\n| Flooring | ⏳ Pending | — | $1,500 |\n\n## Bathroom\n\n- [ ] Replace vanity\n- [ ] New mirror + lighting\n- [ ] Re-grout shower tiles\n- [ ] Install exhaust fan\n- [ ] Paint\n\n## Budget Summary\n\n```\nKitchen budget:  $20,000\nKitchen spent:   $12,600 (includes ordered items)\nKitchen remaining: $7,400\n\nBathroom budget: $5,000\nBathroom spent:  $0\nBathroom remaining: $5,000\n\nTotal budget:    $25,000\nTotal remaining: $12,400\n```\n\n## Timeline\n\nKitchen should be done by mid-July. Bathroom after that (August).",
        ),
        (
            "Python vs Go - Engineering Notes",
            "2025-06-25 14:00:00",
            "# Python vs Go: Engineering Comparison\n\n## Performance\n\n| Benchmark | Python | Go |\n|-----------|--------|----|\n| HTTP req/s (hello world) | ~5,000 | ~80,000 |\n| JSON parse (100KB) | 8ms | 0.4ms |\n| Binary size | N/A (interp) | ~10MB static |\n| Startup time | ~100ms | ~5ms |\n\n## Developer Experience\n\n### Python wins\n- Rich ecosystem (NumPy, Pandas, Django, FastAPI)\n- Dynamic typing = fast prototyping\n- Jupyter notebooks for data exploration\n- Massive community and learning resources\n\n### Go wins\n- Built-in concurrency (goroutines + channels)\n- Static typing catches bugs early\n- Single binary deployment\n- Excellent tooling: `gofmt`, `go test`, `pprof`\n\n## My Recommendation\n\n| Use Case | Pick |\n|----------|------|\n| Data science / ML | Python |\n| High-performance API | Go |\n| CLI tools | Go |\n| Quick scripts | Python |\n| Systems programming | Go |\n\n*They complement each other. Knowing both is a superpower.*",
        ),
        (
            "Podcast Recommendations",
            "2025-06-26 12:00:00",
            "# Podcast List\n\n## Tech\n\n1. **Lex Fridman Podcast** — Long-form interviews with scientists, engineers, and thinkers. Recent favorite: the episode with John Carmack on AI and VR.\n2. **Software Engineering Daily** — Deep dives into infrastructure and architecture. Great for learning about distributed systems.\n3. **The Bike Shed** — Thoughtbot's podcast on Ruby, Rails, and software development practices.\n\n## Finance\n\n4. **Planet Money** — Economics explained in 15-20 minutes. Surprisingly engaging.\n5. **ChooseFI** — Financial independence and early retirement.\n\n## Science\n\n6. **Radiolab** — Mind-blowing stories about science and philosophy.\n7. **Hidden Brain** — Psychology and human behavior.\n\n## Just for Fun\n\n8. **99% Invisible** — Design and architecture stories.\n9. **No Such Thing as a Fish** — Weird facts from the QI elves.\n10. **Conan O'Brien Needs a Friend** — Hilarious interviews with celebrities.\n\n## Current Rotation\n\nThis week I'm catching up on Lex Fridman (the Nick Bostrom episode was incredible) and some old Radiolab episodes about the science of music.",
        ),
        (
            "First Aid & Emergency Kit",
            "2025-06-27 18:30:00",
            "# Emergency Preparedness\n\n## First Aid Kit\n\n### Essentials\n\n- [x] Bandages (various sizes)\n- [x] Sterile gauze pads\n- [x] Medical tape\n- [x] Antiseptic wipes\n- [x] Antibiotic ointment\n- [x] Hydrocortisone cream\n- [x] Tweezers\n- [x] Scissors\n- [x] Disposable gloves\n- [x] CPR mask\n- [ ] Tourniquet (need to buy)\n- [x] Instant cold packs\n- [x] Burn cream\n- [x] Pain relievers (ibuprofen, acetaminophen)\n- [x] Antihistamines (Benadryl)\n\n### Additional\n\n- Emergency blanket\n- Flashlight + extra batteries\n- Whistle\n- Water purification tablets\n- Lighter\n\n## Emergency Plan\n\n1. Check scene safety before helping\n2. Call 911 for life-threatening emergencies\n3. Know the address of your location\n4. Keep emergency contacts in phone + written copy\n\n> \"The goal of first aid is not to treat, but to stabilize until professional help arrives.\"",
        ),
        (
            "Conference Talk Ideas",
            "2025-06-28 08:00:00",
            "# Talk Ideas for RustConf 2025\n\n## Idea 1: Building CLI Tools That Don't Suck\n\nHow to create ergonomic, fast, and user-friendly command-line applications in Rust.\n\n**Outline:**\n\n1. Argument parsing with `clap` (derive API)\n2. Progress bars with `indicatif`\n3. Colors and formatting with `colored` or `owo-colors`\n4. Configuration handling with `directories` crate\n5. Cross-platform distribution\n\n## Idea 2: Safe Systems Programming Patterns\n\nPractical patterns for writing safe, concurrent systems code in Rust.\n\n- Typestate pattern for protocol handling\n- Arena allocation with lifetimes\n- Lock-free data structures with atomics\n- Zero-copy parsing with `nom` or `winnow`\n\n## Idea 3: From Prototype to Production\n\nMigrating a Python/Django monolith to Rust microservices.\n\n- Why Rust was worth the rewrite\n- Migration strategy (strangler fig pattern)\n- Performance: 10x throughput, 100x less memory\n- Lessons learned\n\n## Submission Deadline\n\nCFP closes August 1st. Need to submit at least 2 proposals by then.\n\n*Which one should I prioritize? I think Idea 1 is most accessible for a general audience.*",
        ),
        (
            "Movie Watchlist 2025",
            "2025-06-29 21:00:00",
            "# Movie Watchlist\n\n## Must Watch\n\n- [ ] Dune: Part Two — Heard the visual effects are groundbreaking\n- [x] The Substance — Demi Moore is incredible. Body horror at its finest.\n- [ ] Anora — Won Palme d'Or at Cannes 2024\n- [ ] Challengers — Tennis drama with a killer soundtrack\n- [x] Furiosa: A Mad Max Saga — Anya Taylor-Joy kills it. Less action than Fury Road, more world-building.\n- [ ] The Brutalist — 3.5 hour epic about an architect\n- [x] Nosferatu — Robert Eggers remake. Gothic, atmospheric, terrifying.\n\n## Recently Watched\n\n### The Substance ★★★★½\nBrilliant satire about beauty standards. Coralie Fargeat's direction is unhinged in the best way. The last 30 minutes are pure chaos.\n\n### Furiosa ★★★★\nDoesn't reach Fury Road's heights, but fills in the backstory beautifully. Chris Hemsworth is surprisingly good as the villain.\n\n### Nosferatu ★★★★\nLily-Rose Depp is phenomenal. The cinematography is haunting. Eggers proves he's the master of period horror.\n\n## Ratings Scale\n\n★★★★★ = Masterpiece\n★★★★½ = Excellent\n★★★★ = Great\n★★★½ = Good\n★★★ = Decent\n★★½ = Mediocre\n★★ = Bad\n★½ = Terrible\n★ = Offensive",
        ),
        (
            "Learning Plan - System Design",
            "2025-06-30 06:00:00",
            "# System Design Learning Plan\n\n## Week 1-2: Foundations\n\n- [x] Read \"Designing Data-Intensive Applications\" (Ch 1-4)\n- [ ] Read DDIA Ch 5-8\n- [ ] Watch MIT 6.824 lectures (distributed systems)\n- [ ] Practice: Design a URL shortener\n\n## Week 3-4: Core Concepts\n\n- [ ] Consistency models (strong, eventual, causal)\n- [ ] Consensus algorithms (Paxos, Raft)\n- [ ] Partitioning and replication strategies\n- [ ] Practice: Design a chat system\n\n## Week 5-6: Advanced Topics\n\n- [ ] Stream processing (Kafka, Flink)\n- [ ] Distributed transactions (2PC, Sagas)\n- [ ] CDN and caching strategies\n- [ ] Practice: Design Uber\n\n## Week 7-8: Mock Interviews\n\n- [ ] Design YouTube\n- [ ] Design Twitter\n- [ ] Design a key-value store\n- [ ] Design a rate limiter\n\n## Resources\n\n| Resource | Price | Notes |\n|----------|-------|-------|\n| DDIA book | $45 | Must-read |\n| Grokking System Design | $80 | Good structure |\n| System Design Interview (Alex Xu) | $35 | Good for interview prep |\n| YouTube: ByteByteGo | Free | Great animations |\n\n*Target: Ready for system design interviews by September.*",
        ),
        (
            "Weekend Hike - Mount Tamalpais",
            "2025-07-01 07:00:00",
            "# Mount Tamalpais Hike\n\n**Date**: Saturday, July 1\n**Trail**: Dipsea → Steep Ravine → Matt Davis loop\n**Distance**: 7.2 miles\n**Elevation**: 1,800 ft\n**Time**: 4h 15m (with breaks)\n\n## Trail Notes\n\nStarted at Stinson Beach around 7:30 AM. The climb up Dipsea is relentless — 600 ft in the first mile. But the views of the Pacific are worth every step.\n\nSteep Ravine is the highlight. Wooden ladders bolted into the cliff face next to a waterfall. Feels like a tropical jungle for a brief section.\n\nMatt Davis trail back down offers sweeping views of the coast. Wildflowers are in full bloom — lupine, poppies, and some sort of purple flower I couldn't identify.\n\n## What I Packed\n\n- 2L water (drank all of it)\n- PB&J sandwich\n- Apple\n- Trail mix\n- Sunscreen\n- Windbreaker (needed at the summit)\n\n## Photos\n\nTook about 30 photos. Best one is from the summit looking south toward the Golden Gate Bridge.\n\n*Definitely doing this again. Next time: Muir Woods + Mount Tam loop.*",
        ),
        (
            "Side Project Ideas",
            "2025-07-02 22:00:00",
            "# Side Project Ideas\n\n## Current Projects\n\n### 1. TUI Todo App\nA terminal-based todo list with markdown notes. Written in Rust with ratatui.\n- Active development\n- Need to add: recurring todos, tags, search filtering\n\n### 2. Personal Dashboard\nStatic site with weather, calendar, stocks, and todo sync. Astro + React.\n- Waiting on API keys for weather data\n\n### 3. RSS Reader\nSelf-hosted RSS reader with AI summarization.\n- Backend in Go, frontend in Svelte\n- Paused — need to design the database schema\n\n## New Ideas\n\n### Price Tracker\nTrack prices on Amazon/eBay and get notifications when items drop below a threshold.\n\n### Recipe Manager\nStore, tag, and search recipes. Import from URL. Meal planner.\n\n### Habit Tracker\nSimple streak-based habit tracker. Minimalist, no gamification.\n\n## Decision Matrix\n\n| Idea | Interest | Difficulty | Time | Score |\n|------|----------|------------|------|-------|\n| Price Tracker | 7/10 | 4/10 | 2 weeks | ⭐⭐⭐ |\n| Recipe Manager | 8/10 | 5/10 | 3 weeks | ⭐⭐⭐⭐ |\n| Habit Tracker | 6/10 | 3/10 | 1 week | ⭐⭐ |\n\n*Recipe manager wins. Starting next weekend.*",
        ),
        (
            "Git Advanced Workflows",
            "2025-07-03 16:00:00",
            "# Advanced Git\n\n## Interactive Rebase\n\n```bash\ngit rebase -i HEAD~5\n```\n\nCommands:\n- `pick` — use commit\n- `reword` — change message\n- `squash` — combine with previous\n- `fixup` — squash, discard message\n- `drop` — remove commit\n\n## Bisect\n\nFind the commit that introduced a bug:\n\n```bash\ngit bisect start\ngit bisect bad          # Current version is broken\ngit bisect good v1.0    # v1.0 was working\n# Git checks out the midpoint; tell it good/bad\n# Repeat until the culprit is found\ngit bisect reset\n```\n\n## Cherry-pick\n\n```bash\ngit cherry-pick <commit-hash>\ngit cherry-pick -n <hash>  # Don't auto-commit\n```\n\n## Reflog\n\nYour safety net:\n\n```bash\ngit reflog              # Show all HEAD movements\ngit reset HEAD@{2}      # Go back to where you were 2 moves ago\n```\n\n## Worktrees\n\nWork on multiple branches simultaneously:\n\n```bash\ngit worktree add ../feature-branch feature-branch\ngit worktree list\ngit worktree remove ../feature-branch\n```\n\n## Aliases\n\n```bash\ngit config --global alias.lg \"log --graph --oneline --all\"\ngit config --global alias.undo \"reset --soft HEAD~1\"\n```",
        ),
        (
            "Cooking Notes - Thai Curry",
            "2025-07-04 19:00:00",
            "# Thai Green Curry\n\n## Ingredients\n\n### Curry Paste (makes ~4 servings)\n\n- 4-6 green bird's eye chilies\n- 2 shallots, chopped\n- 4 cloves garlic\n- 1 stalk lemongrass (white part only)\n- 1 inch galangal (or ginger)\n- 1 tsp shrimp paste\n- 1 tsp coriander seeds, toasted\n- 1 tsp cumin seeds, toasted\n- Zest of 1 kaffir lime\n- 1 bunch cilantro stems\n- 1 tsp white pepper\n\n### Curry\n\n- 1 can (400ml) coconut milk\n- 1 lb chicken thighs, sliced\n- 1 cup Thai eggplant, quartered\n- 1 cup bamboo shoots\n- 3-4 kaffir lime leaves\n- 1 tbsp fish sauce\n- 1 tsp palm sugar (or brown sugar)\n- Thai basil leaves for garnish\n\n## Method\n\n1. Toast coriander and cumin seeds, then grind\n2. Pound all paste ingredients in mortar (or blitz in food processor)\n3. Heat half the coconut milk in a wok. Fry 3 tbsp paste until fragrant\n4. Add chicken, cook until sealed\n5. Add remaining coconut milk, vegetables, and kaffir lime leaves\n6. Simmer 10 minutes\n7. Season with fish sauce and sugar\n8. Garnish with Thai basil and sliced chili\n\n## Notes\n\nHomemade paste is significantly better than store-bought. Freezes well — portion into ice cube trays.\n\n*Pair with: jasmine rice and a crisp lager.*",
        ),
        (
            "DevOps Automation Ideas",
            "2025-07-05 10:00:00",
            "# DevOps Automation\n\n## Current Setup\n\n- **CI**: GitHub Actions (build, test, lint, deploy)\n- **Infra**: Terraform on AWS (ECS, RDS, ElastiCache)\n- **Monitoring**: CloudWatch + PagerDuty\n- **Logging**: ELK stack\n\n## Automation Targets\n\n### 1. Automated DB Backups\n\nWeekly EBS snapshots + daily pg_dump to S3. Retention: 30 days.\n\n### 2. Zero-downtime Deployments\n\nBlue-green deployment with ECS. Health check → drain old → route new.\n\n```yaml\n# Terraform blue-green config\nblue:\n  desired_count: 2\n  task_definition: app:1\ngreen:\n  desired_count: 2\n  task_definition: app:2\n```\n\n### 3. Self-healing\n\nAuto-restart failed tasks. Scale up on high CPU. Scale to zero overnight.\n\n### 4. Cost Optimization\n\n| Action | Savings |\n|--------|---------|\n| Right-size RDS (db.t3.medium → db.t3.small) | $40/mo |\n| Delete unused EBS volumes | ~$15/mo |\n| S3 lifecycle (move infrequent to Glacier) | ~$10/mo |\n| Reserved instances (1yr, partial) | ~30% off |\n\n*Total potential savings: ~$80-100/month*",
        ),
        (
            "Mindfulness & Meditation Log",
            "2025-07-06 21:30:00",
            "# Meditation Log\n\n## July Check-in\n\nStarted a daily meditation practice 45 days ago. Using the Waking Up app by Sam Harris.\n\n## Progress\n\n| Week | Sessions | Avg Duration | Notes |\n|------|----------|-------------|-------|\n| 1 | 7 | 10 min | Hard to focus |\n| 2 | 6 | 12 min | Slightly easier |\n| 3 | 7 | 15 min | Had a few \"good\" sits |\n| 4 | 5 | 10 min | Missed two days |\n| 5 | 7 | 15 min | Back on track |\n| 6 | 7 | 18 min | Starting to notice benefits |\n\n## Observations\n\n- My mind wanders less during work\n- I notice when I'm getting anxious earlier\n- Sleep quality has improved\n- I'm more patient with people\n\n## Techniques I've Tried\n\n1. **Focused attention** — breath counting, body scan\n2. **Open awareness** — noting thoughts without engaging\n3. **Loving-kindness** — metta practice (hardest but most rewarding)\n\n> \"You should sit in meditation for 20 minutes every day — unless you're too busy; then you should sit for an hour.\" — Old Zen saying\n\n*Goal: Get to 20 min daily by end of July. Add metta practice twice a week.*",
        ),
    ];

    // ── TODOS ──────────────────────────────────────────────────────
    let mut todos = Vec::new();

    let descriptions = vec![
        // Work / project tasks
        "Finish user dashboard redesign mockup",
        "Implement payment webhook handler for Stripe",
        "Write integration tests for authentication flow",
        "Deploy staging environment to Kubernetes",
        "Set up CI/CD pipeline for backend services",
        "Migrate legacy database to new schema",
        "Review pull request #342 for API changes",
        "Write documentation for the onboarding flow",
        "Set up monitoring dashboard in Grafana",
        "Fix memory leak in WebSocket connection handler",
        "Add rate limiting to public API endpoints",
        "Update dependencies to latest versions",
        "Refactor OrderProcessor class into smaller services",
        "Add input validation to all user-facing forms",
        "Implement search functionality with Elasticsearch",
        "Configure log aggregation with ELK stack",
        "Set up automated database backups to S3",
        "Create Terraform module for ECS service",
        "Write postmortem for last week's outage",
        "Update incident response runbook",
        "Run load tests on the new API gateway",
        "Configure CORS headers for frontend requests",
        "Implement dark mode toggle in settings",
        "Add keyboard shortcuts for power users",
        "Set up feature flags with LaunchDarkly",
        "Implement OAuth2 integration with Google and GitHub",
        "Add export to CSV functionality",
        "Create admin dashboard for user management",
        "Fix pagination bug on search results page",
        "Add WebSocket reconnection with exponential backoff",
        "Implement file upload with progress indicator",
        "Set up email notification service with SendGrid",
        "Add two-factor authentication support",
        "Create API rate limiting dashboard",
        "Fix timezone handling in date picker component",
        "Add drag-and-drop support for kanban board",
        "Implement session timeout and auto-logout",
        "Set up A/B testing pipeline",
        "Add tooltips to all icon buttons",
        "Implement undo/redo for text editor",
        "Add autocomplete to search input",
        "Create responsive email templates",
        "Fix accessibility issues in navigation menu",
        "Add breadcrumb navigation throughout app",
        "Implement bulk operations for list views",
        "Set up error tracking with Sentry",
        "Add loading skeletons for all data tables",
        "Implement keyboard-only navigation mode",
        "Create user preference management page",
        "Fix cross-browser compatibility issues",
        "Add offline support with service workers",
        "Implement PDF invoice generation",
        "Set up content delivery network with CloudFront",
        "Add activity log for audit trail",
        "Implement collaborative editing with WebSocket",
        "Create onboarding tutorial for new users",
        "Fix sticky header positioning on scroll",
        "Add citation modal for referencing sources",
        "Implement attachment preview for images",
        "Set up automated dependency updates with Dependabot",
        "Add backup verification script",
        "Implement database read replicas for reporting",
        "Set up VPN access for remote team",
        "Create employee onboarding checklist",
        "Update company wiki with architecture docs",
        "Review and update incident management runbook",
        "Renew SSL certificates before expiry",
        // Personal / health
        "Go for a morning run (5K)",
        "Do 30 pushups and 30 situps",
        "Drink 8 glasses of water today",
        "Meal prep for the week",
        "Try new Thai curry recipe",
        "Go to yoga class at 6 PM",
        "Stretch for 10 minutes after waking up",
        "Take vitamins with breakfast",
        "Schedule annual physical checkup",
        "Walk 10,000 steps today",
        "Try intermittent fasting 16:8",
        "Do a 15-minute meditation session",
        "Track macros in MyFitnessPal",
        "Go for a bike ride on Saturday",
        "Do a plank challenge (increase by 10s)",
        "Take a cold shower in the morning",
        "Try a new sport this month",
        "Get 7+ hours of sleep tonight",
        "Reduce screen time 30 min before bed",
        // Home
        "Water all indoor plants",
        "Repot the monstera into a larger container",
        "Clean the gutters this weekend",
        "Fix the leaky kitchen faucet",
        "Paint the living room accent wall",
        "Organize the garage storage shelves",
        "Change HVAC filter",
        "Deep clean the refrigerator",
        "Sell unused furniture on Craigslist",
        "Install smart thermostat",
        "Replace bathroom caulking",
        "Trim the hedges in the backyard",
        "Set up automatic bill payments",
        "Check smoke detector batteries",
        "Clean out and organize the pantry",
        "Wash windows inside and out",
        "Replace worn-out doormat",
        "Compile home inventory for insurance",
        "Set up composting bin",
        "Fix the squeaky bedroom door",
        // Finance
        "Review monthly budget vs actual spending",
        "Rebalance investment portfolio",
        "Increase 401(k) contribution by 2%",
        "Open a Roth IRA account",
        "Research high-yield savings accounts",
        "Cancel unused subscription services",
        "Set up automatic monthly investments",
        "File quarterly estimated taxes",
        "Check credit score and report",
        "Negotiate better credit card rewards",
        "Create emergency fund tracking spreadsheet",
        "Review insurance coverage (health, auto, home)",
        "Set up 529 plan for education savings",
        "Make a will or update existing one",
        "Research mortgage refinance options",
        "Track net worth this quarter",
        "Compare car insurance quotes",
        "Create sinking funds for known expenses",
        "Set up budgeting software (YNAB or similar)",
        "Review recurring charges on bank statement",
        // Learning
        "Read chapter 5 of Designing Data-Intensive Applications",
        "Complete Rustlings exercises 15-25",
        "Watch MIT 6.824 lecture on Raft consensus",
        "Practice system design: design a URL shortener",
        "Complete Kubernetes tutorial on KodeKloud",
        "Read AWS Well-Architected Framework whitepaper",
        "Learn vim keybindings for VS Code",
        "Finish Python crash course on Coursera",
        "Read about event sourcing pattern",
        "Practice SQL window functions",
        "Explore Apache Kafka documentation",
        "Write a blog post about Rust ownership",
        "Complete Terraform associate certification prep",
        "Study for AWS Solutions Architect exam",
        "Read Clean Code chapters 7-10",
        "Follow Go Tour of the tutorial",
        "Practice data structures on LeetCode",
        "Read about CQRS pattern implementation",
        "Learn basic Japanese phrases for trip",
        "Watch conference talks from RustConf 2024",
        // Social
        "Call mom and check in",
        "Catch up with college friends over dinner",
        "Plan birthday party for partner",
        "Reply to Sarah's email about the wedding",
        "Schedule game night with friends",
        "Write thank-you note to mentor",
        "Attend local meetup this week",
        "Volunteer at the community garden",
        "Plan weekend BBQ with neighbors",
        "Send holiday cards to extended family",
        "Organize team lunch next week",
        "Join a book club",
        "Reach out to former colleagues on LinkedIn",
        "Plan hiking trip with friends",
        "Schedule coffee chat with industry peers",
        "Attend company all-hands meeting",
        "Host a dinner party this weekend",
        "Help friend move to new apartment",
        "Send birthday gift to niece",
        "Participate in community cleanup event",
        // Errands
        "Pick up dry cleaning",
        "Get oil change for the car",
        "Renew driver's license online",
        "Pick up prescription from pharmacy",
        "Return Amazon package at UPS",
        "Schedule dentist appointment",
        "Buy groceries for the week",
        "Get haircut before the trip",
        "Drop off donation items at Goodwill",
        "Pick up birthday cake from bakery",
        "Go to post office for shipping label",
        "Get car smog check",
        "Borrow books from library",
        "Print photos for the album",
        "Order contact lenses",
        "Get keys copied at hardware store",
        "Buy wrapping paper and card",
        "Drop off recycling at center",
        "Exchange defective item at store",
        "Get passport photos taken",
        // Misc / fun
        "Update personal website portfolio",
        "Back up photos to external drive and cloud",
        "Organize digital music collection",
        "Create photo album from Japan trip",
        "Set up home media server with Jellyfin",
        "Update resume and LinkedIn profile",
        "Tune up the bicycle (chain, brakes, tires)",
        "Start a habit tracker journal",
        "Learn to make sourdough bread from starter",
        "Set up home automation routines",
        "Digitize old family photos",
        "Create a capsule wardrobe",
        "Plan vacation itinerary for fall trip",
        "Start a compost pile in the backyard",
        "Build a raised garden bed for vegetables",
        "Learn to knit or crochet",
        "Try geocaching this weekend",
        "Set up a bird feeder and identify species",
        "Create a reading list for the year",
        "Learn basic lock picking as a hobby",
    ];

    // Generate todos with dates spread across June-July 2025
    let base = NaiveDateTime::parse_from_str("2025-05-20 08:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let mut seq: u64 = 0;

    for (i, desc) in descriptions.iter().enumerate() {
        let offset_hours = i as i64 * 6; // one todo every 6 hours
        let mut dt = base + chrono::Duration::hours(offset_hours);
        // add a random-ish minute offset
        dt = dt.with_minute((i * 17 % 60) as u32).unwrap();

        let done = match i % 10 {
            0 | 1 | 2 | 5 => true, // some done
            9 => true,             // done
            _ => false,            // pending
        };

        let archived = match i % 13 {
            7 | 11 => true,
            _ => false,
        };

        todos.push(Todo {
            id: seed_id(dt.and_utc().timestamp_millis(), seq),
            description: desc.to_string(),
            done,
            archived,
            created_at: dt,
        });
        seq += 1;
    }

    // ── WRITE FILES ────────────────────────────────────────────────
    let notes: Vec<Note> = notes_data
        .into_iter()
        .enumerate()
        .map(|(i, (title, date_str, content))| {
            let dt = NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").unwrap();
            let dt = dt.with_minute((i * 11 % 60) as u32).unwrap();
            Note {
                id: seed_id(dt.and_utc().timestamp_millis(), i as u64),
                title: title.to_string(),
                content: content.to_string(),
                created_at: dt,
                updated_at: dt,
            }
        })
        .collect();

    fs::write(&notes_path, serde_json::to_string_pretty(&notes).unwrap())
        .expect("Failed to write notes file");

    fs::write(&todos_path, serde_json::to_string_pretty(&todos).unwrap())
        .expect("Failed to write todos file");

    println!("✅ Seeded {} notes → {}", notes.len(), notes_path.display());
    println!("✅ Seeded {} todos → {}", todos.len(), todos_path.display());
    println!(
        "   done: {}, pending: {}, archived: {}",
        todos.iter().filter(|t| t.done).count(),
        todos.iter().filter(|t| !t.done && !t.archived).count(),
        todos.iter().filter(|t| t.archived).count(),
    );
}
