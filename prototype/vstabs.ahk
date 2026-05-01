#Requires AutoHotkey v2.0
#SingleInstance Force

; ============================================================================
; vstabs v0.0 — AHK prototype
; L1 project tab bar for desktop VS Code (local / WSL / SSH).
; UX sanity check before committing to the Tauri MVP. Hardcoded project list.
; ============================================================================


; ---- Project list ----------------------------------------------------------
;
; Edit to match your projects. Order in this array == tab order
; (and Ctrl+Alt+N hotkey index).
;
;   env "local" — local Windows folder (uses path directly)
;   env "wsl"   — WSL distro + POSIX path
;   env "ssh"   — VS Code Remote-SSH host + POSIX path

global PROJECTS := [
    Map(
        "name",   "sample-local",
        "env",    "local",
        "path",   "C:\Projects\sample-local",
        "icon",   "🏠"
    ),
    Map(
        "name",   "sample-wsl",
        "env",    "wsl",
        "distro", "Ubuntu",
        "path",   "/home/your-user/sample-wsl",
        "icon",   "🐧"
    ),
    Map(
        "name",     "sample-remote",
        "env",      "ssh",
        "ssh_host", "my-remote-host",
        "path",     "/home/your-user",
        "icon",     "🖥"
    )
]


; ---- Visuals ---------------------------------------------------------------

global TAB_HEIGHT := 30
global FONT_SIZE  := 10
global BAR_BG_HEX := "1e1e1e"


; ---- State -----------------------------------------------------------------

global g_gui     := ""
global g_visible := true


; ---- GUI build -------------------------------------------------------------

BuildTabBar() {
    global g_gui

    g_gui := Gui("+AlwaysOnTop +ToolWindow -Caption +E0x08000000")  ; WS_EX_NOACTIVATE
    g_gui.BackColor := BAR_BG_HEX
    g_gui.SetFont("s" FONT_SIZE " cFFFFFF", "Segoe UI")
    g_gui.MarginX := 4
    g_gui.MarginY := 4

    btnHeight := TAB_HEIGHT - 4
    x := 4
    for i, proj in PROJECTS {
        label := proj["icon"] " " proj["name"] " " EnvTag(proj["env"])
        ; Coarse width estimate; button text auto-clips if too tight.
        w := StrLen(label) * 9 + 16
        btn := g_gui.Add("Button", "x" x " y2 w" w " h" btnHeight, label)
        btn.OnEvent("Click", ClickHandler(i))
        x += w + 2
    }

    totalW := x + 2
    MonitorGetWorkArea(MonitorGetPrimary(), &mLeft, &mTop, &mRight, &mBottom)
    barX := mLeft + (mRight - mLeft - totalW) // 2
    g_gui.Show("x" barX " y" mTop " w" totalW " h" TAB_HEIGHT " NoActivate")
}

ClickHandler(index) {
    return (*) => ActivateProject(index)
}

EnvTag(env) {
    switch env {
        case "local": return "[L]"
        case "wsl":   return "[W]"
        case "ssh":   return "[S]"
    }
    return ""
}


; ---- Activate or spawn -----------------------------------------------------

ActivateProject(index) {
    proj := PROJECTS[index]
    hwnd := FindVSCodeWindow(proj)
    if hwnd {
        if WinGetMinMax("ahk_id " hwnd) = -1
            WinRestore("ahk_id " hwnd)
        WinActivate("ahk_id " hwnd)
        return
    }
    Run(BuildLaunchCommand(proj), , "Hide")
    SetTitleMatchMode 2
    pattern := BaseName(proj["path"]) " "
    if WinWait(pattern, , 15) {
        WinActivate(pattern)
    } else {
        TrayTip("Window for " proj["name"] " did not appear within 15s.", "vstabs")
    }
}

BuildLaunchCommand(proj) {
    base := 'cmd.exe /c code'
    switch proj["env"] {
        case "local":
            return base ' "' proj["path"] '"'
        case "wsl":
            return base ' --remote wsl+' proj["distro"] ' "' proj["path"] '"'
        case "ssh":
            return base ' --remote ssh-remote+' proj["ssh_host"] ' "' proj["path"] '"'
    }
    throw ValueError("Unknown env: " proj["env"])
}

FindVSCodeWindow(proj) {
    ; VS Code window titles observed:
    ;   "<folder> [WSL: Ubuntu] - Visual Studio Code"
    ;   "<folder> [SSH: host] - Visual Studio Code"
    ;   "<folder> - Visual Studio Code"
    SetTitleMatchMode 2
    folder := BaseName(proj["path"])
    envMarker := ""
    switch proj["env"] {
        case "wsl": envMarker := "[WSL: " proj["distro"] "]"
        case "ssh": envMarker := "[SSH: " proj["ssh_host"] "]"
    }
    ids := WinGetList("Visual Studio Code")
    for hwnd in ids {
        title := WinGetTitle("ahk_id " hwnd)
        if !InStr(title, folder)
            continue
        if envMarker = "" {
            if !InStr(title, "[WSL:") && !InStr(title, "[SSH:")
                return hwnd
        } else if InStr(title, envMarker) {
            return hwnd
        }
    }
    return 0
}

BaseName(path) {
    p := RTrim(path, "/\")
    if RegExMatch(p, "[^/\\]+$", &m)
        return m[0]
    return p
}


; ---- Hotkeys ---------------------------------------------------------------

; Toggle tab bar
^#Space:: ToggleBar()

ToggleBar() {
    global g_visible
    if g_visible {
        g_gui.Hide()
        g_visible := false
    } else {
        g_gui.Show("NoActivate")
        g_visible := true
    }
}

; Ctrl+Alt+1..9 → activate project N
Loop 9 {
    HotKey("^!" A_Index, NumberHotkey)
}

NumberHotkey(thisHotkey) {
    idx := Integer(SubStr(thisHotkey, 0))
    if idx >= 1 && idx <= PROJECTS.Length
        ActivateProject(idx)
}

; Reload (handy while tweaking)
^!r:: Reload()


; ---- Boot ------------------------------------------------------------------

BuildTabBar()
TrayTip("Ctrl+Win+Space toggle, Ctrl+Alt+N select, Ctrl+Alt+R reload.", "vstabs v0.0")
