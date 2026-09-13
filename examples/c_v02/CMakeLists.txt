cmake_minimum_required(VERSION 3.16)
project(OpenUIV02Examples LANGUAGES C CXX)

find_package(OpenUI 0.2 CONFIG REQUIRED)

foreach(example IN ITEMS hello counter todo dashboard)
  add_executable(openui_${example} ${example}.c)
  target_link_libraries(openui_${example} PRIVATE OpenUI::OpenUI)
endforeach()

add_executable(openui_header_smoke header_smoke.cc)
target_link_libraries(openui_header_smoke PRIVATE OpenUI::OpenUI)
