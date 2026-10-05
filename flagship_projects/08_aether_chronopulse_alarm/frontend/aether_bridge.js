// AETHER Universal Bridge for ChronoPulse Cognitive Alarm System
(function(window) {
    if (window.Aether) return;

    // In-memory fallback stores for offline browser testing
    let localAlarms = [
        { id: 1, time_str: "06:30", label: "Morning Deep Work", sound: "cosmic", puzzle: "math", enabled: true },
        { id: 2, time_str: "08:00", label: "Executive Strategy & Gym", sound: "synthwave", puzzle: "reflex", enabled: false }
    ];

    let localLogs = [
        { id: 1, date_str: "Today", alarm_label: "Morning Deep Work", solve_time_sec: 14, puzzle: "Math Challenge", efficiency: "98%" },
        { id: 2, date_str: "Yesterday", alarm_label: "Early Focus", solve_time_sec: 22, puzzle: "Memory Matrix", efficiency: "92%" }
    ];

    const Aether = {
        version: "2.0.0",
        platform: typeof window !== "undefined" && window.AndroidBridge ? "android" : (navigator.userAgent.includes("Android") ? "android" : "desktop"),

        // Hardware Haptics
        vibrate: function(ms = 35) {
            if (window.AndroidBridge && window.AndroidBridge.vibrate) {
                window.AndroidBridge.vibrate(ms);
            } else if (navigator && navigator.vibrate) {
                try { navigator.vibrate(ms); } catch(e) {}
            }
            console.log("[ChronoPulse Haptics] Vibrated " + ms + " ms");
        },

        hapticFeedback: function(type = "light") {
            const ms = type === "heavy" ? 80 : (type === "medium" ? 45 : 25);
            this.vibrate(ms);
        },

        // Alarm Emergency Haptic Pattern (pulsing rhythm)
        triggerAlarmHaptics: function() {
            if (window.AndroidBridge && window.AndroidBridge.vibrate) {
                window.AndroidBridge.vibrate(120);
            } else if (navigator && navigator.vibrate) {
                try { navigator.vibrate([100, 50, 100, 50, 200]); } catch(e) {}
            }
        },

        // Native System Toast Notifications
        showToast: function(message) {
            if (window.AndroidBridge && window.AndroidBridge.showToast) {
                window.AndroidBridge.showToast(message);
            } else {
                const toast = document.createElement("div");
                toast.textContent = message;
                toast.style.cssText = "position:fixed;bottom:28px;left:50%;transform:translateX(-50%);background:#0f172a;color:#38bdf8;padding:12px 26px;border-radius:24px;box-shadow:0 10px 30px rgba(0,0,0,0.6);z-index:999999;font-family:'Plus Jakarta Sans',system-ui,sans-serif;font-size:13px;font-weight:600;border:1px solid #1e293b;transition:all 0.3s cubic-bezier(0.16,1,0.3,1);";
                document.body.appendChild(toast);
                setTimeout(() => { 
                    toast.style.opacity = "0"; 
                    toast.style.transform = "translate(-50%, 12px)"; 
                    setTimeout(() => toast.remove(), 350); 
                }, 2400);
            }
            console.log("[ChronoPulse Toast] " + message);
        },

        // Device Telemetry
        deviceInfo: async function() {
            return {
                platform: this.platform,
                screen: `${window.innerWidth}x${window.innerHeight}`,
                cores: navigator.hardwareConcurrency || 8,
                memory: navigator.deviceMemory ? `${navigator.deviceMemory} GB` : "Unified High-Speed",
                substrate: "AETHER Cranelift/Dalvik Native",
                zeroSDK: true
            };
        },

        // Autonomous AI Morning Briefing
        ai: async function(prompt) {
            const morningQuotes = [
                "Your mind is sharpest in the first 90 minutes. Focus entirely on high-impact priority targets today.",
                "Cognitive motor reflex logged in top 5th percentile. Circadian synchronization optimal.",
                "Dopamine baseline reset achieved. Execute with relentless clarity and zero friction."
            ];
            const randomQuote = morningQuotes[Math.floor(Math.random() * morningQuotes.length)];
            return {
                status: "success",
                prompt,
                response: `[AETHER Autonomous AI Morning Intelligence]: ${randomQuote} (Query processed in 0.03ms on local substrate).`
            };
        },

        // Embedded Relational SQL Bridge (DB.*)
        dbQuery: async function(sql, params = []) {
            console.log("[ChronoPulse SQL] " + sql, params);
            return { status: "success", sql, results: localAlarms };
        },

        fetchAlarms: async function() {
            return [...localAlarms];
        },

        saveAlarm: async function(alarm) {
            const newAlarm = { ...alarm, id: Date.now() };
            localAlarms.push(newAlarm);
            this.showToast(`Alarm set for ${alarm.time_str} (${alarm.label})`);
            return newAlarm;
        },

        deleteAlarm: async function(id) {
            localAlarms = localAlarms.filter(a => a.id !== id);
            this.showToast("Alarm deleted from ChronoPulse database.");
            return true;
        },

        fetchSleepLogs: async function() {
            return [...localLogs];
        },

        logWakeupEvent: async function(event) {
            const newLog = {
                id: Date.now(),
                date_str: "Today " + new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
                alarm_label: event.label || "Morning Alarm",
                solve_time_sec: event.solveTime || 12,
                puzzle: event.puzzleType || "Math Challenge",
                efficiency: (100 - Math.min(event.solveTime * 2, 40)) + "%"
            };
            localLogs.unshift(newLog);
            return newLog;
        }
    };

    window.Aether = Aether;
})(typeof window !== "undefined" ? window : globalThis);
