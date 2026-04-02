// Copyright 2025 The Open UI Authors
// SPDX-License-Identifier: BSD-3-Clause
//
// openui_init.h — Blink runtime bootstrap (extracted from rendering_test.cc).

#ifndef OPENUI_OPENUI_INIT_H_
#define OPENUI_OPENUI_INIT_H_

#include "openui/openui.h"

// Initialize the blink rendering runtime. Must be called once before any
// other oui_* function. Thread-safe for the first call; subsequent calls
// return OUI_ERROR_ALREADY_INITIALIZED.
OuiStatus openui_runtime_init(const OuiInitConfig* config);

// Shut down the blink rendering runtime. After this call, no oui_* function
// may be called. Currently a best-effort cleanup.
void openui_runtime_shutdown();

// Returns true if the runtime has been initialized.
bool openui_runtime_is_initialized();

// Returns true if a test harness provides the task environment (so documents
// should NOT create their own).
bool openui_runtime_has_external_task_env();

// Mark the runtime as initialized externally (for test harnesses that
// bootstrap blink themselves). Also sets the external-task-env flag.
void openui_runtime_mark_initialized_externally();

#endif  // OPENUI_OPENUI_INIT_H_
