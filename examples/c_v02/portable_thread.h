// Minimal pthread_create/pthread_join for the wrong-thread guard consumers.
// POSIX hosts use <pthread.h> directly; Windows maps the same calls onto
// CreateThread so each consumer keeps one source for both platforms.
#ifndef OPENUI_EXAMPLES_PORTABLE_THREAD_H
#define OPENUI_EXAMPLES_PORTABLE_THREAD_H

#ifdef _WIN32
#include <stdlib.h>
#include <windows.h>

typedef HANDLE pthread_t;

typedef struct {
  void* (*start)(void*);
  void* argument;
} openui_example_thread_start;

static DWORD WINAPI openui_example_thread_main(LPVOID parameter) {
  openui_example_thread_start start = *(openui_example_thread_start*)parameter;
  free(parameter);
  start.start(start.argument);
  return 0;
}

static int pthread_create(pthread_t* thread,
                          const void* attributes,
                          void* (*start)(void*),
                          void* argument) {
  (void)attributes;
  openui_example_thread_start* parameter = (openui_example_thread_start*)malloc(sizeof(*parameter));
  if (!parameter) {
    return 1;
  }
  parameter->start = start;
  parameter->argument = argument;
  *thread = CreateThread(NULL, 0, openui_example_thread_main, parameter, 0, NULL);
  if (!*thread) {
    free(parameter);
    return 1;
  }
  return 0;
}

static int pthread_join(pthread_t thread, void** result) {
  if (result) {
    *result = NULL;
  }
  if (WaitForSingleObject(thread, INFINITE) != WAIT_OBJECT_0) {
    return 1;
  }
  return CloseHandle(thread) ? 0 : 1;
}
#else
#include <pthread.h>
#endif

#endif  // OPENUI_EXAMPLES_PORTABLE_THREAD_H
