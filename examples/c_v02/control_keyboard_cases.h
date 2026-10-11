/* Generated native operations; no expected states or script interpreter. */
#ifndef OPENUI_CONTROL_KEYBOARD_CASES_H_
#define OPENUI_CONTROL_KEYBOARD_CASES_H_
#include <stddef.h>

#include "openui.h"
enum KeyboardOperationKind {
  KEY_ATTRIBUTE,
  KEY_CHECKED,
  KEY_FOCUS,
  KEY_DETACH,
  KEY_STYLE,
  KEY_DISPATCH
};
typedef struct KeyboardOperation {
  int kind, target, checked, down, code, modifiers;
  const char *name, *value, *json;
} KeyboardOperation;
typedef struct KeyboardCase {
  const char *name, *control;
  const KeyboardOperation *pre, *steps, *handler_ops;
  size_t pre_count, step_count, handler_count;
  unsigned handler_event;
  int handler_target, prevent;
} KeyboardCase;
static const KeyboardOperation keyboard_0_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_1_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_2_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_3_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_4_pre[] = {
    {KEY_CHECKED, 4, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const KeyboardOperation keyboard_4_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_5_pre[] = {
    {KEY_CHECKED, 4, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const KeyboardOperation keyboard_5_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_6_pre[] = {
    {KEY_CHECKED, 4, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const KeyboardOperation keyboard_6_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_7_pre[] = {
    {KEY_CHECKED, 4, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const KeyboardOperation keyboard_7_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_8_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_8_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_9_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_9_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_10_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_10_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_11_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_11_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_12_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "display", "none",
     "{\"name\":\"display\",\"op\":\"style\",\"target\":\"b\",\"value\":\"none\"}"},
};
static const KeyboardOperation keyboard_12_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_13_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "display", "none",
     "{\"name\":\"display\",\"op\":\"style\",\"target\":\"b\",\"value\":\"none\"}"},
};
static const KeyboardOperation keyboard_13_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_14_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "display", "none",
     "{\"name\":\"display\",\"op\":\"style\",\"target\":\"b\",\"value\":\"none\"}"},
};
static const KeyboardOperation keyboard_14_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_15_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "display", "none",
     "{\"name\":\"display\",\"op\":\"style\",\"target\":\"b\",\"value\":\"none\"}"},
};
static const KeyboardOperation keyboard_15_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_16_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "visibility", "hidden",
     "{\"name\":\"visibility\",\"op\":\"style\",\"target\":\"b\",\"value\":\"hidden\"}"},
};
static const KeyboardOperation keyboard_16_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_17_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "visibility", "hidden",
     "{\"name\":\"visibility\",\"op\":\"style\",\"target\":\"b\",\"value\":\"hidden\"}"},
};
static const KeyboardOperation keyboard_17_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_18_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "visibility", "hidden",
     "{\"name\":\"visibility\",\"op\":\"style\",\"target\":\"b\",\"value\":\"hidden\"}"},
};
static const KeyboardOperation keyboard_18_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_19_pre[] = {
    {KEY_STYLE, 4, 0, 0, 0, 0, "visibility", "hidden",
     "{\"name\":\"visibility\",\"op\":\"style\",\"target\":\"b\",\"value\":\"hidden\"}"},
};
static const KeyboardOperation keyboard_19_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_20_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "tabindex", "-1",
     "{\"name\":\"tabindex\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"-1\"}"},
};
static const KeyboardOperation keyboard_20_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_21_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "tabindex", "-1",
     "{\"name\":\"tabindex\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"-1\"}"},
};
static const KeyboardOperation keyboard_21_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_22_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "tabindex", "-1",
     "{\"name\":\"tabindex\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"-1\"}"},
};
static const KeyboardOperation keyboard_22_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_23_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "tabindex", "-1",
     "{\"name\":\"tabindex\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"-1\"}"},
};
static const KeyboardOperation keyboard_23_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_24_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "form", "fb",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"fb\"}"},
};
static const KeyboardOperation keyboard_24_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_25_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "form", "fb",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"fb\"}"},
};
static const KeyboardOperation keyboard_25_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_26_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "form", "fb",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"fb\"}"},
};
static const KeyboardOperation keyboard_26_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_27_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "form", "fb",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"fb\"}"},
};
static const KeyboardOperation keyboard_27_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_28_pre[] = {
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_28_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_29_pre[] = {
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_29_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_30_pre[] = {
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_30_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_31_pre[] = {
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_31_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_32_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"other\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"other\"}"},
};
static const KeyboardOperation keyboard_32_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 0, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_33_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"other\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"other\"}"},
};
static const KeyboardOperation keyboard_33_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 38, 0, "ArrowUp", NULL,
     "{\"code\":38,\"key\":\"ArrowUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_34_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"other\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"other\"}"},
};
static const KeyboardOperation keyboard_34_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_35_pre[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"other\"}"},
    {KEY_ATTRIBUTE, 5, 0, 0, 0, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"c\",\"value\":\"other\"}"},
};
static const KeyboardOperation keyboard_35_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 40, 0, "ArrowDown", NULL,
     "{\"code\":40,\"key\":\"ArrowDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_36_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 1, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":1,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 1, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":1,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_37_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 1, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":1,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 1, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":1,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_38_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 2, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":2,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 2, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":2,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_39_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 2, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":2,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 2, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":2,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_40_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 4, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":4,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 4, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":4,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_41_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 4, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":4,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 4, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":4,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_42_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 8, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":8,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 8, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":8,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_43_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 8, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":8,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 8, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":8,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_44_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 3, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":3,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 3, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":3,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_45_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 3, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":3,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 3, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":3,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_46_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 5, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":5,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 5, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":5,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_47_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 5, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":5,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 5, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":5,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_48_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 10, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":10,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 10, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":10,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_49_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 10, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":10,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 10, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":10,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_50_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 37, 15, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":15,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 37, 15, "ArrowLeft", NULL,
     "{\"code\":37,\"key\":\"ArrowLeft\",\"modifiers\":15,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_51_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 15, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":15,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 15, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":15,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_52_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_52_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 36, 0, "Home", NULL,
     "{\"code\":36,\"key\":\"Home\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 36, 0, "Home", NULL,
     "{\"code\":36,\"key\":\"Home\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_53_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_53_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 36, 0, "Home", NULL,
     "{\"code\":36,\"key\":\"Home\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 36, 0, "Home", NULL,
     "{\"code\":36,\"key\":\"Home\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_54_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_54_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 35, 0, "End", NULL,
     "{\"code\":35,\"key\":\"End\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 35, 0, "End", NULL,
     "{\"code\":35,\"key\":\"End\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_55_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_55_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 35, 0, "End", NULL,
     "{\"code\":35,\"key\":\"End\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 35, 0, "End", NULL,
     "{\"code\":35,\"key\":\"End\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_56_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_56_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 33, 0, "PageUp", NULL,
     "{\"code\":33,\"key\":\"PageUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 33, 0, "PageUp", NULL,
     "{\"code\":33,\"key\":\"PageUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_57_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_57_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 33, 0, "PageUp", NULL,
     "{\"code\":33,\"key\":\"PageUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 33, 0, "PageUp", NULL,
     "{\"code\":33,\"key\":\"PageUp\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_58_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_58_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 34, 0, "PageDown", NULL,
     "{\"code\":34,\"key\":\"PageDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 34, 0, "PageDown", NULL,
     "{\"code\":34,\"key\":\"PageDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_59_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_59_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 34, 0, "PageDown", NULL,
     "{\"code\":34,\"key\":\"PageDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 34, 0, "PageDown", NULL,
     "{\"code\":34,\"key\":\"PageDown\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_60_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_60_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_61_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_61_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_62_pre[] = {
    {KEY_CHECKED, 3, 1, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const KeyboardOperation keyboard_62_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_63_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_63_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_64_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_65_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_66_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_67_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_68_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 13, 0, "Enter", NULL,
     "{\"code\":13,\"key\":\"Enter\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_69_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_70_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_70_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_71_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_71_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_72_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_72_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_73_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_73_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_FOCUS, 4, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"b\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_74_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_74_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DETACH, 3, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"a\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_75_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_75_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_76_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_76_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_77_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_77_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_78_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_78_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_79_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_79_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_80_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_80_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_FOCUS, 4, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"b\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_81_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_81_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DETACH, 3, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"a\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_82_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_82_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_83_pre[] = {
    {KEY_CHECKED, 3, 0, 0, 0, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
};
static const KeyboardOperation keyboard_83_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_84_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_85_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_86_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_87_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_FOCUS, 4, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"b\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_88_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DETACH, 3, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"a\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_89_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_ATTRIBUTE, 3, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_90_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 1, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 32, 0, " ", NULL,
     "{\"code\":32,\"key\":\" \",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_91_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_91_handler[] = {
    {KEY_FOCUS, 5, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"c\"}"},
};
static const KeyboardOperation keyboard_92_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_92_handler[] = {
    {KEY_DETACH, 3, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"a\"}"},
};
static const KeyboardOperation keyboard_93_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_94_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_95_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_95_handler[] = {
    {KEY_FOCUS, 5, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"c\"}"},
};
static const KeyboardOperation keyboard_96_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_96_handler[] = {
    {KEY_FOCUS, 5, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"focus\",\"target\":\"c\"}"},
};
static const KeyboardOperation keyboard_97_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_97_handler[] = {
    {KEY_ATTRIBUTE, 4, 0, 0, 0, 0, "disabled", "",
     "{\"name\":\"disabled\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
};
static const KeyboardOperation keyboard_98_steps[] = {
    {KEY_DISPATCH, -1, 0, 1, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"down\"}"},
    {KEY_DISPATCH, -1, 0, 0, 39, 0, "ArrowRight", NULL,
     "{\"code\":39,\"key\":\"ArrowRight\",\"modifiers\":0,\"op\":\"key\",\"phase\":\"up\"}"},
};
static const KeyboardOperation keyboard_98_handler[] = {
    {KEY_DETACH, 4, 0, 0, 0, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"b\"}"},
};
static const KeyboardCase keyboard_cases[] = {
    {"plain-ArrowLeft", "radio", NULL, keyboard_0_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"plain-ArrowUp", "radio", NULL, keyboard_1_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"plain-ArrowRight", "radio", NULL, keyboard_2_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"plain-ArrowDown", "radio", NULL, keyboard_3_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"other-checked-ArrowLeft", "radio", keyboard_4_pre, keyboard_4_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"other-checked-ArrowUp", "radio", keyboard_5_pre, keyboard_5_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"other-checked-ArrowRight", "radio", keyboard_6_pre, keyboard_6_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"other-checked-ArrowDown", "radio", keyboard_7_pre, keyboard_7_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"disabled-peer-ArrowLeft", "radio", keyboard_8_pre, keyboard_8_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"disabled-peer-ArrowUp", "radio", keyboard_9_pre, keyboard_9_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"disabled-peer-ArrowRight", "radio", keyboard_10_pre, keyboard_10_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"disabled-peer-ArrowDown", "radio", keyboard_11_pre, keyboard_11_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"display-none-peer-ArrowLeft", "radio", keyboard_12_pre, keyboard_12_steps, NULL, 1, 2, 0, 0,
     -1, 0},
    {"display-none-peer-ArrowUp", "radio", keyboard_13_pre, keyboard_13_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"display-none-peer-ArrowRight", "radio", keyboard_14_pre, keyboard_14_steps, NULL, 1, 2, 0, 0,
     -1, 0},
    {"display-none-peer-ArrowDown", "radio", keyboard_15_pre, keyboard_15_steps, NULL, 1, 2, 0, 0,
     -1, 0},
    {"visibility-hidden-peer-ArrowLeft", "radio", keyboard_16_pre, keyboard_16_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"visibility-hidden-peer-ArrowUp", "radio", keyboard_17_pre, keyboard_17_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"visibility-hidden-peer-ArrowRight", "radio", keyboard_18_pre, keyboard_18_steps, NULL, 1, 2,
     0, 0, -1, 0},
    {"visibility-hidden-peer-ArrowDown", "radio", keyboard_19_pre, keyboard_19_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"negative-tabindex-peer-ArrowLeft", "radio", keyboard_20_pre, keyboard_20_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"negative-tabindex-peer-ArrowUp", "radio", keyboard_21_pre, keyboard_21_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"negative-tabindex-peer-ArrowRight", "radio", keyboard_22_pre, keyboard_22_steps, NULL, 1, 2,
     0, 0, -1, 0},
    {"negative-tabindex-peer-ArrowDown", "radio", keyboard_23_pre, keyboard_23_steps, NULL, 1, 2, 0,
     0, -1, 0},
    {"other-form-peer-ArrowLeft", "radio", keyboard_24_pre, keyboard_24_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"other-form-peer-ArrowUp", "radio", keyboard_25_pre, keyboard_25_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"other-form-peer-ArrowRight", "radio", keyboard_26_pre, keyboard_26_steps, NULL, 1, 2, 0, 0,
     -1, 0},
    {"other-form-peer-ArrowDown", "radio", keyboard_27_pre, keyboard_27_steps, NULL, 1, 2, 0, 0, -1,
     0},
    {"empty-name-ArrowLeft", "radio", keyboard_28_pre, keyboard_28_steps, NULL, 3, 2, 0, 0, -1, 0},
    {"empty-name-ArrowUp", "radio", keyboard_29_pre, keyboard_29_steps, NULL, 3, 2, 0, 0, -1, 0},
    {"empty-name-ArrowRight", "radio", keyboard_30_pre, keyboard_30_steps, NULL, 3, 2, 0, 0, -1, 0},
    {"empty-name-ArrowDown", "radio", keyboard_31_pre, keyboard_31_steps, NULL, 3, 2, 0, 0, -1, 0},
    {"sole-group-ArrowLeft", "radio", keyboard_32_pre, keyboard_32_steps, NULL, 2, 2, 0, 0, -1, 0},
    {"sole-group-ArrowUp", "radio", keyboard_33_pre, keyboard_33_steps, NULL, 2, 2, 0, 0, -1, 0},
    {"sole-group-ArrowRight", "radio", keyboard_34_pre, keyboard_34_steps, NULL, 2, 2, 0, 0, -1, 0},
    {"sole-group-ArrowDown", "radio", keyboard_35_pre, keyboard_35_steps, NULL, 2, 2, 0, 0, -1, 0},
    {"modifier-1-ArrowLeft", "radio", NULL, keyboard_36_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-1-ArrowRight", "radio", NULL, keyboard_37_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-2-ArrowLeft", "radio", NULL, keyboard_38_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-2-ArrowRight", "radio", NULL, keyboard_39_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-4-ArrowLeft", "radio", NULL, keyboard_40_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-4-ArrowRight", "radio", NULL, keyboard_41_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-8-ArrowLeft", "radio", NULL, keyboard_42_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-8-ArrowRight", "radio", NULL, keyboard_43_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-3-ArrowLeft", "radio", NULL, keyboard_44_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-3-ArrowRight", "radio", NULL, keyboard_45_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-5-ArrowLeft", "radio", NULL, keyboard_46_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-5-ArrowRight", "radio", NULL, keyboard_47_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-10-ArrowLeft", "radio", NULL, keyboard_48_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-10-ArrowRight", "radio", NULL, keyboard_49_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-15-ArrowLeft", "radio", NULL, keyboard_50_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"modifier-15-ArrowRight", "radio", NULL, keyboard_51_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"special-True-36", "radio", keyboard_52_pre, keyboard_52_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-36", "radio", keyboard_53_pre, keyboard_53_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-True-35", "radio", keyboard_54_pre, keyboard_54_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-35", "radio", keyboard_55_pre, keyboard_55_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-True-33", "radio", keyboard_56_pre, keyboard_56_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-33", "radio", keyboard_57_pre, keyboard_57_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-True-34", "radio", keyboard_58_pre, keyboard_58_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-34", "radio", keyboard_59_pre, keyboard_59_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-True-13", "radio", keyboard_60_pre, keyboard_60_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-13", "radio", keyboard_61_pre, keyboard_61_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-True-32", "radio", keyboard_62_pre, keyboard_62_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"special-False-32", "radio", keyboard_63_pre, keyboard_63_steps, NULL, 1, 2, 0, 0, -1, 0},
    {"checkbox-32", "checkbox", NULL, keyboard_64_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"checkbox-13", "checkbox", NULL, keyboard_65_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"checkbox-39", "checkbox", NULL, keyboard_66_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"button-32", "button", NULL, keyboard_67_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"button-13", "button", NULL, keyboard_68_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"button-39", "button", NULL, keyboard_69_steps, NULL, 0, 2, 0, 0, -1, 0},
    {"radio-cancel-keydown", "radio", keyboard_70_pre, keyboard_70_steps, NULL, 1, 2, 0,
     OUI_EVENT_KEY_DOWN, 3, 1},
    {"radio-cancel-keyup", "radio", keyboard_71_pre, keyboard_71_steps, NULL, 1, 2, 0,
     OUI_EVENT_KEY_UP, 3, 1},
    {"radio-cancel-click", "radio", keyboard_72_pre, keyboard_72_steps, NULL, 1, 2, 0,
     OUI_EVENT_CLICK, 3, 1},
    {"radio-between-space-focus", "radio", keyboard_73_pre, keyboard_73_steps, NULL, 1, 3, 0, 0, -1,
     0},
    {"radio-between-space-detach", "radio", keyboard_74_pre, keyboard_74_steps, NULL, 1, 3, 0, 0,
     -1, 0},
    {"radio-between-space-attribute", "radio", keyboard_75_pre, keyboard_75_steps, NULL, 1, 3, 0, 0,
     -1, 0},
    {"radio-repeated-space-down", "radio", keyboard_76_pre, keyboard_76_steps, NULL, 1, 3, 0, 0, -1,
     0},
    {"checkbox-cancel-keydown", "checkbox", keyboard_77_pre, keyboard_77_steps, NULL, 1, 2, 0,
     OUI_EVENT_KEY_DOWN, 3, 1},
    {"checkbox-cancel-keyup", "checkbox", keyboard_78_pre, keyboard_78_steps, NULL, 1, 2, 0,
     OUI_EVENT_KEY_UP, 3, 1},
    {"checkbox-cancel-click", "checkbox", keyboard_79_pre, keyboard_79_steps, NULL, 1, 2, 0,
     OUI_EVENT_CLICK, 3, 1},
    {"checkbox-between-space-focus", "checkbox", keyboard_80_pre, keyboard_80_steps, NULL, 1, 3, 0,
     0, -1, 0},
    {"checkbox-between-space-detach", "checkbox", keyboard_81_pre, keyboard_81_steps, NULL, 1, 3, 0,
     0, -1, 0},
    {"checkbox-between-space-attribute", "checkbox", keyboard_82_pre, keyboard_82_steps, NULL, 1, 3,
     0, 0, -1, 0},
    {"checkbox-repeated-space-down", "checkbox", keyboard_83_pre, keyboard_83_steps, NULL, 1, 3, 0,
     0, -1, 0},
    {"button-cancel-keydown", "button", NULL, keyboard_84_steps, NULL, 0, 2, 0, OUI_EVENT_KEY_DOWN,
     3, 1},
    {"button-cancel-keyup", "button", NULL, keyboard_85_steps, NULL, 0, 2, 0, OUI_EVENT_KEY_UP, 3,
     1},
    {"button-cancel-click", "button", NULL, keyboard_86_steps, NULL, 0, 2, 0, OUI_EVENT_CLICK, 3,
     1},
    {"button-between-space-focus", "button", NULL, keyboard_87_steps, NULL, 0, 3, 0, 0, -1, 0},
    {"button-between-space-detach", "button", NULL, keyboard_88_steps, NULL, 0, 3, 0, 0, -1, 0},
    {"button-between-space-attribute", "button", NULL, keyboard_89_steps, NULL, 0, 3, 0, 0, -1, 0},
    {"button-repeated-space-down", "button", NULL, keyboard_90_steps, NULL, 0, 3, 0, 0, -1, 0},
    {"arrow-handler-keydown-a-91", "radio", NULL, keyboard_91_steps, keyboard_91_handler, 0, 2, 1,
     OUI_EVENT_KEY_DOWN, 3, 0},
    {"arrow-handler-keydown-a-92", "radio", NULL, keyboard_92_steps, keyboard_92_handler, 0, 2, 1,
     OUI_EVENT_KEY_DOWN, 3, 0},
    {"arrow-handler-keydown-a-93", "radio", NULL, keyboard_93_steps, NULL, 0, 2, 0,
     OUI_EVENT_KEY_DOWN, 3, 1},
    {"arrow-handler-click-b-94", "radio", NULL, keyboard_94_steps, NULL, 0, 2, 0, OUI_EVENT_CLICK,
     4, 1},
    {"arrow-handler-focus-b-95", "radio", NULL, keyboard_95_steps, keyboard_95_handler, 0, 2, 1,
     OUI_EVENT_FOCUS, 4, 0},
    {"arrow-handler-blur-a-96", "radio", NULL, keyboard_96_steps, keyboard_96_handler, 0, 2, 1,
     OUI_EVENT_BLUR, 3, 0},
    {"arrow-handler-click-b-97", "radio", NULL, keyboard_97_steps, keyboard_97_handler, 0, 2, 1,
     OUI_EVENT_CLICK, 4, 0},
    {"arrow-handler-focus-b-98", "radio", NULL, keyboard_98_steps, keyboard_98_handler, 0, 2, 1,
     OUI_EVENT_FOCUS, 4, 0},
};
#endif
