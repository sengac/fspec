# BUG-411 root cause

Both paths drop an error after logging it: creation proceeds without the credential it tried to resolve, and a default model that failed to persist silently reverts on restart. Fix: propagate both errors (creation fails with the credential error; set_default_model returns Result and its callers surface it).
