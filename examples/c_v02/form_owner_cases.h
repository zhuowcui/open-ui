/* Generated native operation data; contains no expected browser values. */
#ifndef OPENUI_FORM_OWNER_CASES_H_
#define OPENUI_FORM_OWNER_CASES_H_
enum FormOperationKind {
  FORM_ATTRIBUTE,
  FORM_REMOVE_ATTRIBUTE,
  FORM_DETACH,
  FORM_APPEND,
  FORM_INSERT,
  FORM_CHECKED
};
typedef struct FormOperation {
  int kind, target, parent, before, checked;
  const char *name, *value, *json;
} FormOperation;
typedef struct FormCase {
  const char* name;
  const FormOperation *pre, *steps;
  size_t pre_count, step_count;
} FormCase;
static const FormOperation form_case_0_steps[] = {
    {FORM_REMOVE_ATTRIBUTE, 2, -1, -1, 0, "id", NULL,
     "{\"name\":\"id\",\"op\":\"remove_attribute\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_1_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "other",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"other\"}"},
};
static const FormOperation form_case_2_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "fb",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"fb\"}"},
};
static const FormOperation form_case_3_steps[] = {
    {FORM_ATTRIBUTE, 3, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fb\",\"value\":\"fa\"}"},
};
static const FormOperation form_case_4_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "fb",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"fb\"}"},
    {FORM_ATTRIBUTE, 3, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fb\",\"value\":\"fa\"}"},
};
static const FormOperation form_case_5_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"\"}"},
};
static const FormOperation form_case_6_steps[] = {
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "form", "",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
};
static const FormOperation form_case_7_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"\"}"},
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "form", "",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
};
static const FormOperation form_case_8_steps[] = {
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
};
static const FormOperation form_case_9_steps[] = {
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "form", "fb",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"fb\"}"},
};
static const FormOperation form_case_10_pre[] = {
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "form", "new",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"new\"}"},
};
static const FormOperation form_case_10_steps[] = {
    {FORM_ATTRIBUTE, 6, -1, -1, 0, "id", "new",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"newform\",\"value\":\"new\"}"},
    {FORM_APPEND, 6, 0, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"body\",\"target\":\"newform\"}"},
};
static const FormOperation form_case_11_steps[] = {
    {FORM_DETACH, 2, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_12_steps[] = {
    {FORM_DETACH, 2, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"fa\"}"},
    {FORM_APPEND, 2, 0, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"body\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_13_steps[] = {
    {FORM_APPEND, 2, 4, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"parking\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_14_steps[] = {
    {FORM_APPEND, 2, 0, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"body\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_15_steps[] = {
    {FORM_INSERT, 2, 0, 3, 0, NULL, NULL,
     "{\"before\":\"fb\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_16_steps[] = {
    {FORM_INSERT, 2, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_17_steps[] = {
    {FORM_APPEND, 7, 4, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"parking\",\"target\":\"a\"}"},
};
static const FormOperation form_case_18_steps[] = {
    {FORM_APPEND, 1, 4, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"parking\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_19_steps[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_20_steps[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_CHECKED, 7, -1, -1, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_21_steps[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
};
static const FormOperation form_case_22_steps[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_APPEND, 1, 0, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"body\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_23_steps[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_24_pre[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
};
static const FormOperation form_case_24_steps[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_25_steps[] = {
    {FORM_ATTRIBUTE, 5, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"blocker\",\"value\":\"fa\"}"},
    {FORM_INSERT, 5, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"blocker\"}"},
};
static const FormOperation form_case_26_pre[] = {
    {FORM_ATTRIBUTE, 5, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"blocker\",\"value\":\"fa\"}"},
    {FORM_INSERT, 5, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"blocker\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_26_steps[] = {
    {FORM_REMOVE_ATTRIBUTE, 5, -1, -1, 0, "id", NULL,
     "{\"name\":\"id\",\"op\":\"remove_attribute\",\"target\":\"blocker\"}"},
};
static const FormOperation form_case_27_pre[] = {
    {FORM_ATTRIBUTE, 5, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"blocker\",\"value\":\"fa\"}"},
    {FORM_INSERT, 5, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"blocker\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_27_steps[] = {
    {FORM_DETACH, 5, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"blocker\"}"},
};
static const FormOperation form_case_28_pre[] = {
    {FORM_ATTRIBUTE, 5, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"blocker\",\"value\":\"fa\"}"},
    {FORM_INSERT, 5, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"blocker\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_28_steps[] = {
    {FORM_INSERT, 2, 0, 5, 0, NULL, NULL,
     "{\"before\":\"blocker\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_29_steps[] = {
    {FORM_ATTRIBUTE, 3, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fb\",\"value\":\"fa\"}"},
};
static const FormOperation form_case_30_pre[] = {
    {FORM_ATTRIBUTE, 3, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fb\",\"value\":\"fa\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
    {FORM_CHECKED, 8, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const FormOperation form_case_30_steps[] = {
    {FORM_INSERT, 3, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"fb\"}"},
};
static const FormOperation form_case_31_steps[] = {
    {FORM_ATTRIBUTE, 6, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"newform\",\"value\":\"fa\"}"},
    {FORM_INSERT, 6, 0, 2, 0, NULL, NULL,
     "{\"before\":\"fa\",\"op\":\"insert\",\"parent\":\"body\",\"target\":\"newform\"}"},
};
static const FormOperation form_case_32_pre[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
    {FORM_CHECKED, 11, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"e\",\"value\":true}"},
};
static const FormOperation form_case_32_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "other",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"other\"}"},
};
static const FormOperation form_case_33_steps[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
    {FORM_APPEND, 1, 4, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"parking\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_34_steps[] = {
    {FORM_ATTRIBUTE, 5, -1, -1, 0, "id", "fa",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"blocker\",\"value\":\"fa\"}"},
    {FORM_APPEND, 5, 4, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"parking\",\"target\":\"blocker\"}"},
};
static const FormOperation form_case_35_steps[] = {
    {FORM_ATTRIBUTE, 4, -1, -1, 0, "id", "other",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"parking\",\"value\":\"other\"}"},
};
static const FormOperation form_case_36_pre[] = {
    {FORM_APPEND, 11, 1, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"wrap\",\"target\":\"e\"}"},
};
static const FormOperation form_case_36_steps[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_37_pre[] = {
    {FORM_APPEND, 11, 1, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"wrap\",\"target\":\"e\"}"},
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_37_steps[] = {
    {FORM_CHECKED, 7, -1, -1, 0, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":false}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_38_pre[] = {
    {FORM_APPEND, 11, 1, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"wrap\",\"target\":\"e\"}"},
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
};
static const FormOperation form_case_38_steps[] = {
    {FORM_APPEND, 1, 0, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"body\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_39_pre[] = {
    {FORM_APPEND, 11, 1, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"wrap\",\"target\":\"e\"}"},
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
};
static const FormOperation form_case_39_steps[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
};
static const FormOperation form_case_40_pre[] = {
    {FORM_DETACH, 1, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"wrap\"}"},
    {FORM_ATTRIBUTE, 8, -1, -1, 0, "name", "other",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"other\"}"},
};
static const FormOperation form_case_40_steps[] = {
    {FORM_ATTRIBUTE, 8, -1, -1, 0, "name", "g",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"g\"}"},
};
static const FormOperation form_case_41_pre[] = {
    {FORM_APPEND, 1, 2, -1, 0, NULL, NULL,
     "{\"op\":\"append\",\"parent\":\"fa\",\"target\":\"wrap\"}"},
    {FORM_REMOVE_ATTRIBUTE, 7, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"a\"}"},
    {FORM_REMOVE_ATTRIBUTE, 8, -1, -1, 0, "form", NULL,
     "{\"name\":\"form\",\"op\":\"remove_attribute\",\"target\":\"b\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
};
static const FormOperation form_case_41_steps[] = {
    {FORM_DETACH, 2, -1, -1, 0, NULL, NULL, "{\"op\":\"detach\",\"target\":\"fa\"}"},
};
static const FormOperation form_case_42_pre[] = {
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"\"}"},
    {FORM_ATTRIBUTE, 8, -1, -1, 0, "name", "",
     "{\"name\":\"name\",\"op\":\"attribute\",\"target\":\"b\",\"value\":\"\"}"},
    {FORM_CHECKED, 7, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"a\",\"value\":true}"},
    {FORM_CHECKED, 8, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"b\",\"value\":true}"},
};
static const FormOperation form_case_42_steps[] = {
    {FORM_ATTRIBUTE, 2, -1, -1, 0, "id", "other",
     "{\"name\":\"id\",\"op\":\"attribute\",\"target\":\"fa\",\"value\":\"other\"}"},
};
static const FormOperation form_case_43_pre[] = {
    {FORM_CHECKED, 11, -1, -1, 1, NULL, NULL,
     "{\"op\":\"checked\",\"target\":\"e\",\"value\":true}"},
};
static const FormOperation form_case_43_steps[] = {
    {FORM_ATTRIBUTE, 7, -1, -1, 0, "form", "fa",
     "{\"name\":\"form\",\"op\":\"attribute\",\"target\":\"a\",\"value\":\"fa\"}"},
};
static const FormCase form_cases[] = {
    {"remove-form-id", NULL, form_case_0_steps, 0, 1},
    {"rename-form-id", NULL, form_case_1_steps, 0, 1},
    {"rename-to-other-form-id", NULL, form_case_2_steps, 0, 1},
    {"rename-other-to-first-id", NULL, form_case_3_steps, 0, 1},
    {"swap-form-ids", NULL, form_case_4_steps, 0, 2},
    {"empty-form-id", NULL, form_case_5_steps, 0, 1},
    {"empty-control-form-id", NULL, form_case_6_steps, 0, 1},
    {"empty-id-and-association", NULL, form_case_7_steps, 0, 2},
    {"remove-control-form", NULL, form_case_8_steps, 0, 1},
    {"control-form-other", NULL, form_case_9_steps, 0, 1},
    {"missing-form-created", form_case_10_pre, form_case_10_steps, 1, 2},
    {"detach-form", NULL, form_case_11_steps, 0, 1},
    {"detach-reinsert-form", NULL, form_case_12_steps, 0, 2},
    {"move-form-connected", NULL, form_case_13_steps, 0, 1},
    {"append-form-same-parent", NULL, form_case_14_steps, 0, 1},
    {"insert-form-before-sibling", NULL, form_case_15_steps, 0, 1},
    {"insert-form-before-self", NULL, form_case_16_steps, 0, 1},
    {"move-control-connected", NULL, form_case_17_steps, 0, 1},
    {"move-controls-connected", NULL, form_case_18_steps, 0, 1},
    {"detach-controls", NULL, form_case_19_steps, 0, 1},
    {"detach-controls-toggle", NULL, form_case_20_steps, 0, 3},
    {"detach-controls-remove-form", NULL, form_case_21_steps, 0, 3},
    {"reinsert-controls", NULL, form_case_22_steps, 0, 2},
    {"move-controls-into-form", NULL, form_case_23_steps, 0, 1},
    {"detached-controls-into-form", form_case_24_pre, form_case_24_steps, 3, 1},
    {"shadow-with-non-form", NULL, form_case_25_steps, 0, 2},
    {"shadow-remove-id", form_case_26_pre, form_case_26_steps, 3, 1},
    {"shadow-detach", form_case_27_pre, form_case_27_steps, 3, 1},
    {"shadow-reorder", form_case_28_pre, form_case_28_steps, 3, 1},
    {"duplicate-form-id", NULL, form_case_29_steps, 0, 1},
    {"duplicate-form-reorder", form_case_30_pre, form_case_30_steps, 3, 1},
    {"new-form-shadow", NULL, form_case_31_steps, 0, 2},
    {"ancestor-form-id-change", form_case_32_pre, form_case_32_steps, 5, 1},
    {"nested-connected-move", NULL, form_case_33_steps, 0, 2},
    {"non-form-id-move", NULL, form_case_34_steps, 0, 2},
    {"unrelated-id-change", NULL, form_case_35_steps, 0, 1},
    {"detach-three-radios", form_case_36_pre, form_case_36_steps, 1, 1},
    {"detached-three-toggle", form_case_37_pre, form_case_37_steps, 2, 2},
    {"detached-three-connect", form_case_38_pre, form_case_38_steps, 4, 1},
    {"detached-three-form-connect", form_case_39_pre, form_case_39_steps, 4, 1},
    {"detached-name-merge", form_case_40_pre, form_case_40_steps, 2, 1},
    {"detached-ancestor-form", form_case_41_pre, form_case_41_steps, 4, 1},
    {"managed-empty-name", form_case_42_pre, form_case_42_steps, 4, 1},
    {"same-form-attribute", form_case_43_pre, form_case_43_steps, 1, 1},
};
#endif
