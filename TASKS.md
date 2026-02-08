# BrowserOS - Tasks & Demonstrations

Complete guide with step-by-step demonstrations and hands-on tasks for learning BrowserOS. Each task shows exact commands and expected output.

## Prerequisites

Before starting, ensure BrowserOS is running:
1. Open terminal and run: `python -m http.server 8000` from the `web` directory
2. Open browser: `http://localhost:8000`
3. You should see the green terminal prompt: `>`

---

## Task 1: Basic Navigation and Directory Listing

### Objective
Learn how to navigate the filesystem and list directory contents.

### Demo: What You'll See

When the system boots, you'll be at the root directory. Let's explore:

```
> ls /
bin,etc,home,tmp,var

> pwd
/

> ls /bin
bash,cat,echo,ls,mkdir,touch
```

### Step-by-Step Instructions

1. **Type the first command:**
   ```
   > ls /
   ```
   **Expected Output:** You'll see a list of directories at root level separated by commas: `bin,etc,home,tmp,var`

2. **Check your current location:**
   ```
   > pwd
   ```
   **Expected Output:** `/` (the root directory)

3. **List the bin directory:**
   ```
   > ls /bin
   ```
   **Expected Output:** Core system programs: `bash,cat,echo,ls,mkdir,touch`

### What You Learned
- `ls` command lists directory contents
- `pwd` shows your current working directory (working directory)
- `/` is the root directory (like `C:\` on Windows)

---

## Task 2: Creating Directories and Navigating

### Objective
Create folders and navigate through the directory structure.

### Demo: Creating a Project Structure

```
> mkdir /home
> mkdir /home/myproject
> mkdir /home/myproject/src
> mkdir /home/myproject/data
> cd /home/myproject
> pwd
/home/myproject
> ls /home
myproject
> ls /home/myproject
src,data
```

### Step-by-Step Instructions

1. **Create the main folder:**
   ```
   > mkdir /home
   ```
   **Expected Output:** No output means success

2. **Create a project directory:**
   ```
   > mkdir /home/myproject
   ```
   **Expected Output:** No output (command successful)

3. **Create subdirectories:**
   ```
   > mkdir /home/myproject/src
   > mkdir /home/myproject/data
   ```
   **Expected Output:** No output for each command

4. **Navigate into the project:**
   ```
   > cd /home/myproject
   ```
   **Expected Output:** No output (you've moved)

5. **Verify your location:**
   ```
   > pwd
   ```
   **Expected Output:** `/home/myproject`

6. **List parent directory:**
   ```
   > ls /home
   ```
   **Expected Output:** `myproject`

7. **List current directory contents:**
   ```
   > ls /home/myproject
   ```
   **Expected Output:** `src,data`

### Tips
- Use `mkdir` to create folders
- Use `cd` to navigate into folders
- Use `ls` to see what's inside
- `..` is NOT yet supported - provide full paths

---

## Task 3: Creating and Reading Files

### Objective
Create text files and read their contents using echo and cat.

### Demo: Create a Configuration File

```
> mkdir /etc
> echo "BrowserOS Configuration" > /etc/config.sys
> cat /etc/config.sys
BrowserOS Configuration

> echo "Version: 0.2" >> /etc/config.sys
> cat /etc/config.sys
BrowserOS Configuration
Version: 0.2
```

### Step-by-Step Instructions

1. **Create the etc directory:**
   ```
   > mkdir /etc
   ```
   **Expected Output:** No output

2. **Create a config file with initial content:**
   ```
   > echo "BrowserOS Configuration" > /etc/config.sys
   ```
   **Expected Output:** No output (file created with content)

3. **Read the file to verify:**
   ```
   > cat /etc/config.sys
   ```
   **Expected Output:**
   ```
   BrowserOS Configuration
   ```

4. **Append additional content using >>:**
   ```
   > echo "Version: 0.2" >> /etc/config.sys
   ```
   **Expected Output:** No output (content appended)

5. **Read the complete file:**
   ```
   > cat /etc/config.sys
   ```
   **Expected Output:**
   ```
   BrowserOS Configuration
   Version: 0.2
   ```

### Important Symbols
- `>` (single arrow): **Overwrite** file with new content
- `>>` (double arrow): **Append** content to existing file
- Neither operator in echo: **Print to screen** without saving

### Try This
```
> echo "This goes to screen"
This goes to screen

> echo "This saves to file" > /tmp/test.txt
> cat /tmp/test.txt
This saves to file
```

---

## Task 4: Working with Multiple Files

### Objective
Create and manage multiple files in a project directory.

### Demo: Building a Simple Website Structure

```
> mkdir /home/website
> mkdir /home/website/assets
> echo "<h1>Welcome to BrowserOS</h1>" > /home/website/index.html
> echo "body { color: green; }" > /home/website/assets/style.css
> echo "console.log('Hello');" > /home/website/assets/app.js
> ls /home/website
assets,index.html
> cat /home/website/index.html
<h1>Welcome to BrowserOS</h1>
> cat /home/website/assets/style.css
body { color: green; }
```

### Step-by-Step Instructions

1. **Create website directory:**
   ```
   > mkdir /home/website
   ```

2. **Create assets folder:**
   ```
   > mkdir /home/website/assets
   ```

3. **Create HTML file:**
   ```
   > echo "<h1>Welcome to BrowserOS</h1>" > /home/website/index.html
   ```

4. **Create CSS file:**
   ```
   > echo "body { color: green; }" > /home/website/assets/style.css
   ```

5. **Create JavaScript file:**
   ```
   > echo "console.log('Hello');" > /home/website/assets/app.js
   ```

6. **List website contents:**
   ```
   > ls /home/website
   ```
   **Expected Output:** `assets,index.html`

7. **List assets:**
   ```
   > ls /home/website/assets
   ```
   **Expected Output:** `app.js,style.css`

8. **Read each file to verify:**
   ```
   > cat /home/website/index.html
   <h1>Welcome to BrowserOS</h1>

   > cat /home/website/assets/style.css
   body { color: green; }

   > cat /home/website/assets/app.js
   console.log('Hello');
   ```

### What You Built
A complete website directory structure with HTML, CSS, and JavaScript files - all in the BrowserOS filesystem!

---

## Task 5: System Information and Status

### Objective
Learn about the BrowserOS system using system commands.

### Demo: Checking System Info

```
> uname
BrowserOS v0.2 (WASM Research Edition)

> uptime
Uptime: 0h 0m 45s

> ps
PID 0: init (running)

> pwd
/

> clear
(screen clears)
```

### Step-by-Step Instructions

1. **Get operating system information:**
   ```
   > uname
   ```
   **Expected Output:**
   ```
   BrowserOS v0.2 (WASM Research Edition)
   ```

2. **Check how long the system has been running:**
   ```
   > uptime
   ```
   **Expected Output:** (time varies, shows elapsed seconds)
   ```
   Uptime: 0h 0m 23s
   ```

3. **List running processes:**
   ```
   > ps
   ```
   **Expected Output:**
   ```
   PID 0: init (running)
   ```
   *Note: In v0.2, only the init process is shown*

4. **Verify current directory:**
   ```
   > pwd
   ```
   **Expected Output:** `/` or whatever directory you're in

5. **Clear the screen:**
   ```
   > clear
   ```
   **Expected Output:** Terminal screen clears, prompt returns

### What You Learned
- `uname` shows OS name and version
- `uptime` shows how long system is running
- `ps` lists all processes
- `clear` cleans up terminal view

---

## Task 6: Creating a Document Repository

### Objective
Build a document management system with multiple files and folders.

### Demo: Document Organization

```
> mkdir /home/documents
> mkdir /home/documents/notes
> mkdir /home/documents/reports
> echo "Meeting Notes - 2026-02-08" > /home/documents/notes/meeting.txt
> echo "Project Status Report" > /home/documents/reports/status.txt
> echo "Q1 2026 Summary" > /home/documents/reports/q1.txt
> cat /home/documents/notes/meeting.txt
Meeting Notes - 2026-02-08
> ls /home/documents
notes,reports
> ls /home/documents/notes
meeting.txt
> ls /home/documents/reports
q1.txt,status.txt
```

### Step-by-Step Instructions

1. **Create main documents folder:**
   ```
   > mkdir /home/documents
   ```

2. **Create subdirectories:**
   ```
   > mkdir /home/documents/notes
   > mkdir /home/documents/reports
   ```

3. **Create note file:**
   ```
   > echo "Meeting Notes - 2026-02-08" > /home/documents/notes/meeting.txt
   ```

4. **Create report files:**
   ```
   > echo "Project Status Report" > /home/documents/reports/status.txt
   > echo "Q1 2026 Summary" > /home/documents/reports/q1.txt
   ```

5. **List main documents folder:**
   ```
   > ls /home/documents
   ```
   **Expected Output:** `notes,reports`

6. **List notes subdirectory:**
   ```
   > ls /home/documents/notes
   ```
   **Expected Output:** `meeting.txt`

7. **List reports subdirectory:**
   ```
   > ls /home/documents/reports
   ```
   **Expected Output:** `q1.txt,status.txt`

8. **Read a specific report:**
   ```
   > cat /home/documents/reports/status.txt
   ```
   **Expected Output:**
   ```
   Project Status Report
   ```

9. **Navigate and verify:**
   ```
   > cd /home/documents/notes
   > pwd
   /home/documents/notes
   > cat meeting.txt
   Meeting Notes - 2026-02-08
   ```

### Real-World Use Case
This demonstrates how to organize documents like a real filesystem with categories (notes, reports) and access them from anywhere using full paths.

---

## Task 7: Creating a Backup System

### Objective
Learn how to copy and backup important files.

### Demo: Creating Backups

```
> echo "Important Data" > /etc/database.conf
> mkdir /tmp/backups
> echo "Important Data" > /tmp/backups/database.conf.bak
> cat /etc/database.conf
Important Data
> cat /tmp/backups/database.conf.bak
Important Data
> ls /tmp/backups
database.conf.bak
```

### Step-by-Step Instructions

1. **Create original configuration file:**
   ```
   > echo "Important Data" > /etc/database.conf
   ```

2. **Verify original exists:**
   ```
   > cat /etc/database.conf
   ```
   **Expected Output:**
   ```
   Important Data
   ```

3. **Create backup directory:**
   ```
   > mkdir /tmp/backups
   ```

4. **Create backup by writing same content:**
   ```
   > echo "Important Data" > /tmp/backups/database.conf.bak
   ```

5. **Verify backup created:**
   ```
   > cat /tmp/backups/database.conf.bak
   ```
   **Expected Output:**
   ```
   Important Data
   ```

6. **List backups:**
   ```
   > ls /tmp/backups
   ```
   **Expected Output:** `database.conf.bak`

### Backup Strategy
To backup a file in BrowserOS currently:
1. Read with `cat` to see content
2. Write it to a backup location with `echo`
3. Verify with `cat` again

*Future versions will support `cp` command for faster copying*

---

## Task 8: Building a Software Project Structure

### Objective
Create a realistic Rust project directory structure.

### Demo: Project Layout

```
> mkdir /home/projects
> mkdir /home/projects/myapp
> mkdir /home/projects/myapp/src
> mkdir /home/projects/myapp/tests
> mkdir /home/projects/myapp/docs
> echo "[package]" > /home/projects/myapp/Cargo.toml
> echo "name = \"myapp\"" >> /home/projects/myapp/Cargo.toml
> echo "version = \"0.1.0\"" >> /home/projects/myapp/Cargo.toml
> echo "fn main() {}" > /home/projects/myapp/src/main.rs
> cat /home/projects/myapp/Cargo.toml
[package]
name = "myapp"
version = "0.1.0"
> ls /home/projects/myapp
Cargo.toml,docs,src,tests
```

### Step-by-Step Instructions

1. **Create projects root:**
   ```
   > mkdir /home/projects
   ```

2. **Create application folder:**
   ```
   > mkdir /home/projects/myapp
   ```

3. **Create subdirectories:**
   ```
   > mkdir /home/projects/myapp/src
   > mkdir /home/projects/myapp/tests
   > mkdir /home/projects/myapp/docs
   ```

4. **Create Cargo.toml (Rust manifest):**
   ```
   > echo "[package]" > /home/projects/myapp/Cargo.toml
   ```

5. **Add package information (use >> to append):**
   ```
   > echo "name = \"myapp\"" >> /home/projects/myapp/Cargo.toml
   > echo "version = \"0.1.0\"" >> /home/projects/myapp/Cargo.toml
   ```

6. **Create main source file:**
   ```
   > echo "fn main() {}" > /home/projects/myapp/src/main.rs
   ```

7. **Create library file:**
   ```
   > echo "pub mod utils;" > /home/projects/myapp/src/lib.rs
   ```

8. **Create a test file:**
   ```
   > echo "#[test] fn test_example() {}" > /home/projects/myapp/tests/integration_test.rs
   ```

9. **Create documentation:**
   ```
   > echo "# MyApp" > /home/projects/myapp/docs/README.md
   > echo "A sample Rust application" >> /home/projects/myapp/docs/README.md
   ```

10. **List project structure:**
    ```
    > ls /home/projects/myapp
    Cargo.toml,docs,src,tests
    ```

11. **Verify entire structure:**
    ```
    > ls /home/projects/myapp/src
    lib.rs,main.rs
    
    > ls /home/projects/myapp/tests
    integration_test.rs
    
    > cat /home/projects/myapp/Cargo.toml
    [package]
    name = "myapp"
    version = "0.1.0"
    ```

### Professional Structure
You've created a complete Rust project layout:
```
myapp/
├── Cargo.toml           (package manifest)
├── src/                 (source code)
│   ├── main.rs         (executable)
│   └── lib.rs          (library)
├── tests/              (integration tests)
│   └── integration_test.rs
└── docs/               (documentation)
    └── README.md
```

---

## Task 9: System Logs and Monitoring

### Objective
Create a logging system to track system events.

### Demo: Logging System

```
> mkdir /var/logs
> echo "System started at 2026-02-08 10:30:00" > /var/logs/boot.log
> echo "Kernel initialized" >> /var/logs/boot.log
> echo "All systems operational" >> /var/logs/boot.log
> cat /var/logs/boot.log
System started at 2026-02-08 10:30:00
Kernel initialized
All systems operational
> echo "No errors detected" > /var/logs/error.log
> ps > /var/logs/process.snapshot
> ls /var/logs
boot.log,error.log,process.snapshot
```

### Step-by-Step Instructions

1. **Create logs directory:**
   ```
   > mkdir /var/logs
   ```

2. **Create boot log file:**
   ```
   > echo "System started at 2026-02-08 10:30:00" > /var/logs/boot.log
   ```

3. **Append initialization message:**
   ```
   > echo "Kernel initialized" >> /var/logs/boot.log
   ```

4. **Append operational status:**
   ```
   > echo "All systems operational" >> /var/logs/boot.log
   ```

5. **Read complete boot log:**
   ```
   > cat /var/logs/boot.log
   ```
   **Expected Output:**
   ```
   System started at 2026-02-08 10:30:00
   Kernel initialized
   All systems operational
   ```

6. **Create error log:**
   ```
   > echo "No errors detected" > /var/logs/error.log
   ```

7. **Create process snapshot (save ps output):**
   ```
   > ps > /var/logs/process.snapshot
   > cat /var/logs/process.snapshot
   PID 0: init (running)
   ```

8. **Create system info snapshot:**
   ```
   > uname > /var/logs/system.info
   > cat /var/logs/system.info
   BrowserOS v0.2 (WASM Research Edition)
   ```

9. **List all log files:**
   ```
   > ls /var/logs
   ```
   **Expected Output:** `boot.log,error.log,process.snapshot,system.info`

### Log Files Created
- `boot.log` - Startup messages
- `error.log` - Error tracking
- `process.snapshot` - Process listing snapshot
- `system.info` - System information

---

## Task 10: Data Processing and Text Files

### Objective
Create and process structured data files.

### Demo: CSV Data Management

```
> mkdir /home/data
> echo "id,name,email,status" > /home/data/users.csv
> echo "1,Alice,alice@example.com,active" >> /home/data/users.csv
> echo "2,Bob,bob@example.com,active" >> /home/data/users.csv
> echo "3,Charlie,charlie@example.com,inactive" >> /home/data/users.csv
> cat /home/data/users.csv
id,name,email,status
1,Alice,alice@example.com,active
2,Bob,bob@example.com,active
3,Charlie,charlie@example.com,inactive
> echo "Total: 3 users" > /home/data/summary.txt
> ls /home/data
summary.txt,users.csv
```

### Step-by-Step Instructions

1. **Create data directory:**
   ```
   > mkdir /home/data
   ```

2. **Create CSV file with header:**
   ```
   > echo "id,name,email,status" > /home/data/users.csv
   ```

3. **Add first user:**
   ```
   > echo "1,Alice,alice@example.com,active" >> /home/data/users.csv
   ```

4. **Add second user:**
   ```
   > echo "2,Bob,bob@example.com,active" >> /home/data/users.csv
   ```

5. **Add third user:**
   ```
   > echo "3,Charlie,charlie@example.com,inactive" >> /home/data/users.csv
   ```

6. **View complete CSV file:**
   ```
   > cat /home/data/users.csv
   ```
   **Expected Output:**
   ```
   id,name,email,status
   1,Alice,alice@example.com,active
   2,Bob,bob@example.com,active
   3,Charlie,charlie@example.com,inactive
   ```

7. **Create analysis file:**
   ```
   > echo "Total: 3 users" > /home/data/summary.txt
   ```

8. **Add more analysis:**
   ```
   > echo "Active: 2 users" >> /home/data/summary.txt
   > echo "Inactive: 1 user" >> /home/data/summary.txt
   ```

9. **View summary:**
   ```
   > cat /home/data/summary.txt
   ```
   **Expected Output:**
   ```
   Total: 3 users
   Active: 2 users
   Inactive: 1 user
   ```

10. **List data files:**
    ```
    > ls /home/data
    ```
    **Expected Output:** `summary.txt,users.csv`

### Data Format
You created a CSV (Comma-Separated Values) file that could be imported into Excel, databases, or data analysis tools. This demonstrates BrowserOS's ability to handle real data processing tasks.

---

## Quick Reference Guide

### Essential Commands

| Command | Syntax | Example |
|---------|--------|---------|
| List files | `ls [path]` | `ls /home` |
| Print text | `echo TEXT` | `echo Hello` |
| Save to file | `echo TEXT > path` | `echo Hi > /tmp/msg.txt` |
| Append to file | `echo TEXT >> path` | `echo More >> /tmp/msg.txt` |
| Read file | `cat [path]` | `cat /tmp/msg.txt` |
| Create folder | `mkdir [path]` | `mkdir /home/test` |
| Change directory | `cd [path]` | `cd /home` |
| Current path | `pwd` | `pwd` |
| System info | `uname` | `uname` |
| System uptime | `uptime` | `uptime` |
| Process list | `ps` | `ps` |
| Clear screen | `clear` | `clear` |
| Get help | `help` | `help` |

### File Path Rules

```
Absolute paths (start with /):
  /home              ✅ works
  /home/documents    ✅ works
  /tmp/file.txt      ✅ works

Relative paths (don't start with /):
  home               ❌ not supported in v0.2
  ../documents       ❌ not supported in v0.2
  
Current directory shortcut:
  .                  ❌ not available yet
```

### Output Redirection

```
>     Overwrite file (replace all content)
>>    Append to file (add to end)
(no operator)  Print to screen

Examples:
  echo "hello" > /tmp/file.txt      Creates file with "hello"
  echo "more" >> /tmp/file.txt      Adds "more" to file
  echo "test"                       Shows "test" on screen
  cat /tmp/file.txt                 Shows file content on screen
```

---

## Common Issues & Solutions

### Issue: "Command not found"

**Cause:** Typo or command not recognized

**Solution:** Type `help` to see all available commands:
```
> help
=== BrowserOS Shell ===
Available commands:
  ls, cat, echo, touch, mkdir, pwd, cd, uname, uptime, ps, clear, help
```

### Issue: "Path not found" or file/folder doesn't exist

**Cause:** Path doesn't exist yet

**Solution:** Create the path first:
```
> mkdir /home/documents    (if home doesn't exist, create it)
> mkdir /home/documents    (now create subfolder)
```

### Issue: File doesn't have content when read

**Cause:** Used `touch` instead of `echo`

**Solution:** Use `echo` with `>` to create files with content:
```
> touch /tmp/file.txt       (creates empty file)
> echo "content" > /tmp/file.txt    (creates file with content)
```

### Issue: Multiple echo commands overwrite instead of append

**Cause:** Used `>` instead of `>>`

**Solution:** Use `>>` to append:
```
> echo "Line 1" > /tmp/file.txt     (creates file)
> echo "Line 2" >> /tmp/file.txt    (appends to file)
> cat /tmp/file.txt
Line 1
Line 2
```

---

## Advanced Tips

### Create Complex Directory Trees

```
> mkdir /home/project
> mkdir /home/project/src
> mkdir /home/project/src/modules
> mkdir /home/project/build
> mkdir /home/project/docs
> mkdir /home/project/tests
```

### Organize by Date

```
> mkdir /home/backups
> mkdir /home/backups/2026-02-08
> mkdir /home/backups/2026-02-09
> echo "backup data" > /home/backups/2026-02-08/data.bak
```

### Create Documentation

```
> echo "# Project Name" > /home/docs.md
> echo "" >> /home/docs.md
> echo "## Description" >> /home/docs.md
> echo "This is a sample project" >> /home/docs.md
> echo "" >> /home/docs.md
> echo "## Features" >> /home/docs.md
> echo "- Feature 1" >> /home/docs.md
> echo "- Feature 2" >> /home/docs.md
```

### Verify File Integrity

```
> cat /etc/config.sys
(view content)
> cat /tmp/backups/config.sys.bak
(compare with these outputs to ensure they match)
```

---

## Challenge Exercises

### Exercise 1: Create a Library Catalog

Create a book database:
```
> mkdir /home/books
> echo "title,author,year" > /home/books/catalog.csv
> echo "The Rust Book,Steve Klabnik,2018" >> /home/books/catalog.csv
> echo "Systems Programming,Eben Upton,2012" >> /home/books/catalog.csv
> cat /home/books/catalog.csv
```

### Exercise 2: Create a Time-Tracked Log

Create timestamped log entries:
```
> mkdir /var/events
> echo "[2026-02-08 10:00:00] System booted" > /var/events/log.txt
> echo "[2026-02-08 10:05:00] User logged in" >> /var/events/log.txt
> echo "[2026-02-08 10:10:00] File created" >> /var/events/log.txt
> cat /var/events/log.txt
```

### Exercise 3: Build a Configuration Tree

Create nested config directories:
```
> mkdir /etc/config
> mkdir /etc/config/database
> mkdir /etc/config/network
> echo "host=localhost" > /etc/config/database/connection.conf
> echo "port=3306" >> /etc/config/database/connection.conf
> echo "ip=192.168.1.1" > /etc/config/network/eth0.conf
```

---

## Conclusion

You now understand:
- ✅ How to navigate filesystems with `cd` and `pwd`
- ✅ How to list contents with `ls`
- ✅ How to create files and folders with `mkdir` and `echo`
- ✅ How to read files with `cat`
- ✅ How to use `>` and `>>` for file redirection
- ✅ How to organize complex directory structures
- ✅ How to create backups and logs
- ✅ How to work with structured data (CSV)
- ✅ How to simulate real-world systems in BrowserOS

**Next Steps:**
1. Try combining multiple tasks together
2. Create your own project structures
3. Explore the `ARCHITECTURE.md` for technical details
4. Check `EXTENSIONS.md` to learn how to add custom commands

Happy exploring BrowserOS! 🚀
