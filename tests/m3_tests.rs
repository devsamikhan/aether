use aether::vm::run_source;

#[test]
fn test_m3_theme_tonal_palette_generation() {
    let source = r##"
# 1. Dark Theme with cyan seed
let dark_theme = M3.theme("#00F5FF", True)
assert(dark_theme["dark_mode"] == True)
assert(dark_theme["seed"] == "#00F5FF")
assert(Strings.starts_with(dark_theme["primary"], "#"))
assert(Strings.starts_with(dark_theme["surface"], "#"))
assert(Strings.starts_with(dark_theme["outline"], "#"))

# 2. Light Theme with purple seed
let light_theme = M3.theme("#6750A4", False)
assert(light_theme["dark_mode"] == False)
assert(light_theme["seed"] == "#6750A4")
assert(Strings.starts_with(light_theme["primary"], "#"))
assert(light_theme["on_primary"] == "#FFFFFF")
"##;
    let res = run_source(source);
    assert!(res.is_ok(), "M3 theme test failed: {:?}", res.err());
}

#[test]
fn test_m3_typography_and_shape_tokens() {
    let source = r#"
# Typography
let typo = M3.typography
assert(typo["display_large"]["size"] == 57)
assert(typo["title_large"]["size"] == 22)
assert(typo["body_medium"]["size"] == 14)

# Shapes (Corner radiuses in dp)
let shape = M3.shape
assert(shape["none"] == 0)
assert(shape["xs"] == 4)
assert(shape["sm"] == 8)
assert(shape["md"] == 12)
assert(shape["lg"] == 16)
assert(shape["xl"] == 28)
assert(shape["full"] == 9999)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "M3 typography/shape tokens failed: {:?}", res.err());
}

#[test]
fn test_m3_component_factories() {
    let source = r#"
# 1. M3 Cards
let c_elevated = M3.card("elevated", "Daily Steps", "Fitness Goal", "8,641 steps")
assert(c_elevated["component"] == "M3.Card")
assert(c_elevated["type"] == "elevated")
assert(c_elevated["elevation"] == 1)

let c_filled = M3.card("filled", "Quick Summary", "", "All systems operational")
assert(c_filled["type"] == "filled")
assert(c_filled["elevation"] == 0)

# 2. M3 Buttons
let btn_filled = M3.filled_button("Save Changes", "save")
assert(btn_filled["component"] == "M3.FilledButton")
assert(btn_filled["label"] == "Save Changes")
assert(btn_filled["shape"] == "full")

let btn_tonal = M3.tonal_button("Cancel", "")
assert(btn_tonal["component"] == "M3.TonalButton")

let btn_outlined = M3.outlined_button("View Details", "info")
assert(btn_outlined["component"] == "M3.OutlinedButton")

# 3. M3 Floating Action Button (FAB)
let fab = M3.fab("add", "New Goal", True)
assert(fab["component"] == "M3.FAB")
assert(fab["icon"] == "add")
assert(fab["extended"] == True)
assert(fab["elevation"] == 3)

# 4. M3 TopAppBar & NavigationBar
let app_bar = M3.top_app_bar("AetherPocket", "Personal Tracker")
assert(app_bar["component"] == "M3.TopAppBar")
assert(app_bar["title"] == "AetherPocket")

let nav_items = [
    {"label": "Home", "icon": "home", "active": True},
    {"label": "Activity", "icon": "fitness", "active": False},
    {"label": "Profile", "icon": "person", "active": False}
]
let nav = M3.navigation_bar(nav_items)
assert(nav["component"] == "M3.NavigationBar")

# 5. M3 Chips, Badges & Progress Indicators
let chip = M3.chip("Workout", True, "check")
assert(chip["component"] == "M3.Chip")
assert(chip["selected"] == True)

let badge = M3.badge("5", "error")
assert(badge["component"] == "M3.Badge")
assert(badge["value"] == "5")

let pi = M3.progress_indicator(0.85, "linear")
assert(pi["component"] == "M3.ProgressIndicator")
assert(pi["value"] == 0.85)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "M3 component factories test failed: {:?}", res.err());
}

#[test]
fn test_m3_render_preview() {
    let source = r#"
let card = M3.card("elevated", "Goal Streak", "30 Days", "Keep going!")
M3.render_preview(card)

let btn = M3.filled_button("Confirm Payment", "💳")
M3.render_preview(btn)

let fab = M3.fab("add", "New Task", True)
M3.render_preview(fab)

let bar = M3.top_app_bar("Aether Mobile", "Connected")
M3.render_preview(bar)
"#;
    let res = run_source(source);
    assert!(res.is_ok(), "M3 render_preview test failed: {:?}", res.err());
}
