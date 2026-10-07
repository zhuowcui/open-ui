cmake_minimum_required(VERSION 3.16)
project(OpenUIV02Examples LANGUAGES C CXX)

find_package(OpenUI 0.2 CONFIG REQUIRED)

foreach(example IN ITEMS hello counter todo dashboard accessibility geometry scroll_metrics scroll_into_view primitive_styles svg_viewport float_colors fragment_keywords text_content edit_commands)
  add_executable(openui_${example} ${example}.c)
  target_link_libraries(openui_${example} PRIVATE OpenUI::OpenUI)
endforeach()

add_executable(openui_header_smoke header_smoke.cc)
target_link_libraries(openui_header_smoke PRIVATE OpenUI::OpenUI)

add_executable(openui_scroll_metrics_cpp scroll_metrics.cc)
target_link_libraries(openui_scroll_metrics_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_scroll_metrics_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_primitive_styles_cpp primitive_styles.cc)
target_link_libraries(openui_primitive_styles_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_primitive_styles_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_svg_viewport_cpp svg_viewport.cc)
target_link_libraries(openui_svg_viewport_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_svg_viewport_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_float_colors_cpp float_colors.cc)
target_link_libraries(openui_float_colors_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_float_colors_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_fragment_keywords_cpp fragment_keywords.cc)
target_link_libraries(openui_fragment_keywords_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_fragment_keywords_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_text_content_cpp text_content.cc)
target_link_libraries(openui_text_content_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_text_content_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_native_c native/window.c)
target_link_libraries(openui_native_c PRIVATE OpenUI::OpenUI)
add_executable(openui_native_cpp native/window.cc)
target_link_libraries(openui_native_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_native_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)

add_executable(openui_edit_commands_cpp edit_commands.cc)
target_link_libraries(openui_edit_commands_cpp PRIVATE OpenUI::OpenUI)
set_target_properties(openui_edit_commands_cpp PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED YES)
