<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>PHP 动态支持 - Rust Web Server</title>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        :root {
            --cream: #FAF7F0;
            --sand: #E8DCC4;
            --tan: #D4C5A9;
            --brown: #9C8671;
            --dark-brown: #6B5D52;
            --darker-brown: #4A3F35;
        }

        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(135deg, var(--cream) 0%, var(--sand) 50%, var(--cream) 100%);
            min-height: 100vh;
            padding: 2rem;
        }

        .container {
            max-width: 900px;
            margin: 0 auto;
        }

        .card {
            background: rgba(255, 255, 255, 0.92);
            border-radius: 16px;
            padding: 3rem;
            box-shadow: 0 10px 40px rgba(107, 93, 82, 0.15);
            border: 1px solid var(--tan);
        }

        .back-button {
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.8rem 1.6rem;
            background: var(--brown);
            color: white;
            text-decoration: none;
            border-radius: 10px;
            font-weight: 600;
            font-size: 0.95rem;
            transition: all 0.3s ease;
            box-shadow: 0 4px 15px rgba(156, 134, 113, 0.25);
            border: 1px solid var(--tan);
            margin-bottom: 2rem;
        }

        .back-button:hover {
            background: var(--dark-brown);
            transform: translateY(-2px);
            box-shadow: 0 6px 20px rgba(156, 134, 113, 0.35);
        }

        h1 {
            font-family: 'Inter', sans-serif;
            color: var(--darker-brown);
            margin-bottom: 2.5rem;
            font-size: 2.2rem;
            font-weight: 700;
            letter-spacing: -0.02em;
            text-align: center;
        }

        .section {
            background: var(--cream);
            border-radius: 12px;
            padding: 2rem;
            margin-bottom: 2rem;
            border: 1px solid var(--tan);
        }

        .section-title {
            font-family: 'Inter', sans-serif;
            font-size: 1.1rem;
            color: var(--darker-brown);
            margin-bottom: 1.5rem;
            font-weight: 700;
            letter-spacing: -0.01em;
            display: flex;
            align-items: center;
            gap: 0.5rem;
        }

        .section-title::before {
            content: '';
            display: inline-block;
            width: 4px;
            height: 1.2em;
            background: var(--brown);
            border-radius: 2px;
        }

        .info-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 1.5rem;
        }

        .info-item {
            background: linear-gradient(135deg, var(--sand) 0%, var(--tan) 100%);
            padding: 1.8rem;
            border-radius: 12px;
            text-align: center;
            border: 1px solid var(--brown);
            transition: transform 0.3s ease;
        }

        .info-item:hover {
            transform: translateY(-5px);
        }

        .info-label {
            font-family: 'Inter', sans-serif;
            font-size: 0.8rem;
            color: var(--dark-brown);
            margin-bottom: 0.6rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }

        .info-value {
            font-family: 'JetBrains Mono', monospace;
            font-size: 1.1rem;
            color: var(--darker-brown);
            font-weight: 500;
            word-break: break-all;
        }

        .clock-section {
            text-align: center;
            padding: 3rem 2rem;
        }

        .clock-label {
            font-family: 'Inter', sans-serif;
            font-size: 0.9rem;
            color: var(--brown);
            margin-bottom: 1rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 2px;
        }

        .clock-display {
            font-family: 'JetBrains Mono', monospace;
            font-size: 4rem;
            font-weight: 500;
            color: var(--darker-brown);
            letter-spacing: 0.05em;
            text-shadow: 2px 2px 4px rgba(107, 93, 82, 0.1);
        }

        .clock-date {
            font-family: 'Inter', sans-serif;
            font-size: 1.1rem;
            color: var(--brown);
            margin-top: 1rem;
            font-weight: 500;
        }

        .clock-separator {
            display: inline-block;
            color: var(--brown);
            animation: blink 1s step-end infinite;
        }

        @keyframes blink {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.3; }
        }

        .tip-box {
            background: rgba(156, 134, 113, 0.1);
            border-left: 4px solid var(--brown);
            padding: 1.2rem 1.5rem;
            border-radius: 0 8px 8px 0;
            margin-top: 2rem;
        }

        .tip-text {
            font-family: 'Inter', sans-serif;
            font-size: 0.9rem;
            color: var(--dark-brown);
            line-height: 1.6;
        }

        .loading-dots {
            display: inline-block;
        }

        .loading-dots::after {
            content: '';
            animation: dots 1.5s steps(4, end) infinite;
        }

        @keyframes dots {
            0%, 20% { content: ''; }
            40% { content: '.'; }
            60% { content: '..'; }
            80%, 100% { content: '...'; }
        }

        @media (max-width: 768px) {
            .clock-display {
                font-size: 2.5rem;
            }

            .info-grid {
                grid-template-columns: repeat(2, 1fr);
            }

            .card {
                padding: 2rem;
            }

            h1 {
                font-size: 1.8rem;
            }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <a href="/" class="back-button">← 返回首页</a>

            <h1>PHP Dynamic Support</h1>

            <div class="section">
                <div class="section-title">Server Information</div>
                <div class="info-grid">
                    <div class="info-item">
                        <div class="info-label">PHP Version</div>
                        <div class="info-value"><?php echo PHP_VERSION; ?></div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">Server Uptime</div>
                        <div class="info-value" id="uptime">
                            <span class="loading-dots">Calculating</span>
                        </div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">Memory Usage</div>
                        <div class="info-value"><?php echo format_memory(memory_get_usage(true)); ?></div>
                    </div>
                    <div class="info-item">
                        <div class="info-label">SAPI Environment</div>
                        <div class="info-value"><?php echo php_sapi_name(); ?></div>
                    </div>
                </div>
            </div>

            <div class="section clock-section">
                <div class="section-title" style="justify-content: center;">Current Time</div>
                <div class="clock-label">Server Time</div>
                <div class="clock-display" id="clock">
                    <span class="loading-dots">Loading</span>
                </div>
                <div class="clock-date" id="date">Loading date...</div>
            </div>

            <div class="tip-box">
                <div class="tip-text">
                    <strong>提示：</strong>这是一个动态生成的 PHP 页面。每次刷新页面，服务器信息都会从 PHP 解释器实时获取，时钟会每秒自动更新。
                </div>
            </div>
        </div>
    </div>

    <script>
        function formatUptime(seconds) {
            const days = Math.floor(seconds / 86400);
            const hours = Math.floor((seconds % 86400) / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            const secs = seconds % 60;

            let result = [];
            if (days > 0) result.push(days + 'd');
            if (hours > 0) result.push(hours + 'h');
            if (mins > 0) result.push(mins + 'm');
            result.push(secs + 's');

            return result.join(' ');
        }

        function updateClock() {
            const now = new Date();

            const hours = String(now.getHours()).padStart(2, '0');
            const minutes = String(now.getMinutes()).padStart(2, '0');
            const seconds = String(now.getSeconds()).padStart(2, '0');

            document.getElementById('clock').textContent =
                hours + ':' + minutes + ':' + seconds;

            const year = now.getFullYear();
            const month = String(now.getMonth() + 1).padStart(2, '0');
            const day = String(now.getDate()).padStart(2, '0');
            const weekdays = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
            const weekday = weekdays[now.getDay()];

            document.getElementById('date').textContent =
                year + '-' + month + '-' + day + ' ' + weekday;
        }

        function initUptime() {
            const startTime = Date.now();
            const uptimeSeconds = Math.floor(startTime / 1000);
            document.getElementById('uptime').textContent = formatUptime(uptimeSeconds);
        }

        document.addEventListener('DOMContentLoaded', function() {
            updateClock();
            initUptime();

            setInterval(updateClock, 1000);
        });
    </script>
</body>
</html>

<?php
function format_memory($bytes) {
    $units = ['B', 'KB', 'MB', 'GB'];
    $i = 0;
    while ($bytes >= 1024 && $i < count($units) - 1) {
        $bytes /= 1024;
        $i++;
    }
    return sprintf('%.1f %s', $bytes, $units[$i]);
}
?>
