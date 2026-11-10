# The demo runs basalt in tmux, whose top bar captions each chapter with its
# keys. Captions change while hidden, so the tmux prompt never shows.
Hide
Type "tapes/demo/run.sh"
Enter
Sleep 2s
Ctrl+A
Type ':step 01 ; chapter "Browse the vault" ; keys "j k enter"'
Enter
Show
Sleep 1.5s

Type@400ms "jjjjj"
Sleep 0.6s
Enter
Sleep 2s

Hide
Ctrl+A
Type ':step 02 ; chapter "Open notes in tabs" ; keys "enter H L"'
Enter
Show
Sleep 1s

Type "j"
Sleep 0.4s
Enter
Sleep 1.2s
Type "j"
Sleep 0.4s
Enter
Sleep 1.5s
Type "H"
Sleep 1.2s
Type "H"
Sleep 1.5s

Hide
Ctrl+A
Type ':step 03 ; chapter "Read rendered markdown" ; keys "tab ctrl+b ctrl+d"'
Enter
Show
Sleep 1s

Tab
Sleep 0.8s
Ctrl+B
Sleep 1s
Shift+R
Sleep 2s
Ctrl+D
Sleep 2.5s
Ctrl+D
Sleep 2.5s
Ctrl+D
Sleep 3s

Hide
Ctrl+A
Type ':step 04 ; chapter "Jump with the outline" ; keys "ctrl+o k j g"'
Enter
Show
Sleep 1s

Ctrl+O
Sleep 1.2s
Tab
Sleep 0.8s
Type@400ms "kkk"
Sleep 0.6s
Type "g"
Sleep 2s
Type@400ms "jjj"
Sleep 0.6s
Type "g"
Sleep 3s
Shift+Tab
Sleep 0.6s
Ctrl+O
Sleep 1s

Hide
Ctrl+A
Type ':step 05 ; chapter "Search the whole vault" ; keys "space s ctrl+n ctrl+p enter"'
Enter
Show
Sleep 1s

Space
Type "s"
Sleep 1.2s
Type@250ms "column"
Sleep 1.5s
Type@250ms "s form"
Sleep 1.5s
Ctrl+N
Sleep 0.8s
Ctrl+N
Sleep 0.8s
Ctrl+P
Sleep 0.8s
Ctrl+P
Sleep 1s
Enter
Sleep 2.5s

Hide
Ctrl+A
Type ':step 06 ; chapter "Edit with vim motions" ; keys "gg w e ciw u"'
Enter
Show
Sleep 1s

Type "gg"
Sleep 0.8s
Type@500ms "www"
Sleep 0.6s
Type@500ms "eee"
Sleep 0.8s
Type "ciw"
Sleep 0.6s
Type@120ms "giant"
Sleep 0.8s
Escape
Sleep 1.5s
Type "u"
Sleep 1.8s

Hide
Ctrl+A
Type ':step 07 ; chapter "Link notes as you type" ; keys "[[ tab gx"'
Enter
Show
Sleep 1s

Type "$"
Sleep 0.6s
Type "a"
Sleep 0.4s
Type@150ms " See [[Edi"
Sleep 1.8s
Tab
Sleep 1.2s
Escape
Sleep 0.8s
Type "hh"
Sleep 0.6s
Type "gx"
Sleep 2.5s
Type "H"
Sleep 1.5s

Hide
Ctrl+A
Type ':step 08 ; chapter "Select and yank" ; keys "V j y"'
Enter
Show
Sleep 1s

Type "}"
Sleep 0.6s
Type "j"
Sleep 0.6s
Type "V"
Sleep 0.6s
Type "j"
Sleep 0.6s
Type "j"
Sleep 0.8s
Type "y"
Sleep 2.5s

Hide
Ctrl+A
Type ':step 09 ; chapter "Preview themes live" ; keys "space t j k esc"'
Enter
Show
Sleep 1s

Space
Type "t"
Sleep 1.5s
Type "j"
Sleep 1.3s
Type "j"
Sleep 1.3s
Type "j"
Sleep 1.3s
Type "j"
Sleep 1.3s
Type "j"
Sleep 1.3s
Type "j"
Sleep 1.3s
Type "j"
Sleep 2s
Escape
Sleep 1.5s

Hide
Ctrl+A
Type ':step 10 ; chapter "Open the help" ; keys "?"'
Enter
Show
Sleep 1s

Type "?"
Sleep 2.5s
Ctrl+D
Sleep 1.6s
Ctrl+D
Sleep 1.6s
Ctrl+D
Sleep 1.6s
Ctrl+D
Sleep 3s

Hide
Type "?"
Type "q"
Sleep 1s
