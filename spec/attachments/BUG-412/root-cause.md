# BUG-412 root cause

Test-support handle with silent defaults on a poisoned Mutex; a failing test can then report the wrong thing. Fix: surface poisoning (methods returning Result return Err; others log and fail loudly).
