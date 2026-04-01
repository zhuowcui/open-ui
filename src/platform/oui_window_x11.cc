// src/platform/oui_window_x11.cc — X11/GLX windowing backend
//
// Temporary windowing for SP2 — will be replaced by native platform layer in SP6.
// Provides window creation, GL context, event handling for Linux/X11.

#include "include/openui/openui_skia.h"
#include "src/skia/oui_sk_types_internal.h"

#include "include/core/SkColorSpace.h"
#include "include/core/SkSurface.h"

#ifdef SK_GL
#include "include/gpu/ganesh/GrBackendSurface.h"
#include "include/gpu/ganesh/GrDirectContext.h"
#include "include/gpu/ganesh/SkSurfaceGanesh.h"
#include "include/gpu/ganesh/gl/GrGLDirectContext.h"
#include "include/gpu/ganesh/gl/GrGLInterface.h"
#include "include/gpu/ganesh/gl/glx/GrGLMakeGLXInterface.h"
#include "include/gpu/ganesh/gl/GrGLBackendSurface.h"
#include "include/gpu/ganesh/gl/GrGLTypes.h"
#endif

#include <X11/Xlib.h>
#include <X11/Xutil.h>
#include <X11/keysym.h>
#include <GL/gl.h>
#include <GL/glx.h>

#include <cstdio>
#include <cstdlib>
#include <cstring>

struct OuiWindow_t {
    Display* display;
    Window window;
    GLXContext glx_context;
    Colormap colormap;
    Atom wm_delete_message;
    XIM xim;
    XIC xic;
    int width;
    int height;
    float dpi_scale;
    OuiSkBackend backend;

    // Skia GPU state
#ifdef SK_GL
    sk_sp<GrDirectContext> gr_context;
    sk_sp<SkSurface> surface;
#endif

    // Canvas handle (reused across frames)
    OuiSkCanvas_t canvas_handle;
    bool canvas_valid;

    // Pending text input event (generated alongside KeyPress)
    bool has_pending_text_input;
    OuiEvent pending_text_event;
};

static void recreate_surface(OuiWindow_t* win) {
#ifdef SK_GL
    if (!win->gr_context) return;

    // Get the current FBO (0 = default)
    GrGLint fbo_id = 0;
    glGetIntegerv(GL_FRAMEBUFFER_BINDING, &fbo_id);

    GrGLFramebufferInfo fbo_info;
    fbo_info.fFBOID = static_cast<unsigned int>(fbo_id);
    fbo_info.fFormat = GL_RGBA8;
    fbo_info.fProtected = skgpu::Protected::kNo;

    int sample_count = 0;
    glGetIntegerv(GL_SAMPLES, &sample_count);
    int stencil_bits = 0;
    glGetIntegerv(GL_STENCIL_BITS, &stencil_bits);

    auto backend_rt = GrBackendRenderTargets::MakeGL(
        win->width, win->height,
        sample_count > 1 ? sample_count : 0,
        stencil_bits, fbo_info);

    SkColorType color_type = kRGBA_8888_SkColorType;
    win->surface = SkSurfaces::WrapBackendRenderTarget(
        win->gr_context.get(), backend_rt,
        kBottomLeft_GrSurfaceOrigin, color_type, nullptr, nullptr);

    if (win->surface) {
        win->canvas_handle.canvas = win->surface->getCanvas();
        win->canvas_valid = true;
    } else {
        win->canvas_valid = false;
    }
#endif
}

extern "C" {

OuiWindow oui_window_create(
    const char* title, int width, int height, OuiSkBackend backend) {
    if (width <= 0 || height <= 0) return nullptr;
    if (!title) title = "Open UI";

    // For now, only GL backend is supported for windowed rendering
    if (backend == OUI_SK_BACKEND_VULKAN) {
        fprintf(stderr, "oui_window: Vulkan windowing not yet implemented, falling back to GL\n");
        backend = OUI_SK_BACKEND_GL;
    }
    if (backend == OUI_SK_BACKEND_AUTO) {
        backend = OUI_SK_BACKEND_GL;
    }

    Display* display = XOpenDisplay(nullptr);
    if (!display) {
        fprintf(stderr, "oui_window: Cannot open X11 display\n");
        return nullptr;
    }

    int screen = DefaultScreen(display);

    // Choose a GLX visual
    static int visual_attribs[] = {
        GLX_X_RENDERABLE, True,
        GLX_DRAWABLE_TYPE, GLX_WINDOW_BIT,
        GLX_RENDER_TYPE, GLX_RGBA_BIT,
        GLX_X_VISUAL_TYPE, GLX_TRUE_COLOR,
        GLX_RED_SIZE, 8,
        GLX_GREEN_SIZE, 8,
        GLX_BLUE_SIZE, 8,
        GLX_ALPHA_SIZE, 8,
        GLX_DEPTH_SIZE, 0,
        GLX_STENCIL_SIZE, 8,
        GLX_DOUBLEBUFFER, True,
        None
    };

    int num_configs = 0;
    GLXFBConfig* fb_configs = glXChooseFBConfig(
        display, screen, visual_attribs, &num_configs);
    if (!fb_configs || num_configs == 0) {
        fprintf(stderr, "oui_window: No suitable GLX framebuffer config\n");
        XCloseDisplay(display);
        return nullptr;
    }

    GLXFBConfig fb_config = fb_configs[0];
    XVisualInfo* vi = glXGetVisualFromFBConfig(display, fb_config);
    XFree(fb_configs);

    if (!vi) {
        fprintf(stderr, "oui_window: No visual for GLX config\n");
        XCloseDisplay(display);
        return nullptr;
    }

    Colormap colormap = XCreateColormap(
        display, RootWindow(display, screen), vi->visual, AllocNone);

    XSetWindowAttributes swa;
    swa.colormap = colormap;
    swa.event_mask = ExposureMask | KeyPressMask | KeyReleaseMask |
                     ButtonPressMask | ButtonReleaseMask |
                     PointerMotionMask | StructureNotifyMask |
                     FocusChangeMask;

    Window window = XCreateWindow(
        display, RootWindow(display, screen),
        0, 0, width, height, 0,
        vi->depth, InputOutput, vi->visual,
        CWColormap | CWEventMask, &swa);

    XFree(vi);

    if (!window) {
        fprintf(stderr, "oui_window: Cannot create X11 window\n");
        XFreeColormap(display, colormap);
        XCloseDisplay(display);
        return nullptr;
    }

    XStoreName(display, window, title);
    XMapWindow(display, window);

    // WM_DELETE_WINDOW protocol
    Atom wm_delete = XInternAtom(display, "WM_DELETE_WINDOW", False);
    XSetWMProtocols(display, window, &wm_delete, 1);

    // Create GLX context
    GLXContext glx_ctx = glXCreateNewContext(
        display, fb_config, GLX_RGBA_TYPE, nullptr, True);
    if (!glx_ctx) {
        fprintf(stderr, "oui_window: Cannot create GLX context\n");
        XDestroyWindow(display, window);
        XFreeColormap(display, colormap);
        XCloseDisplay(display);
        return nullptr;
    }

    glXMakeCurrent(display, window, glx_ctx);

    // Build OuiWindow
    auto* win = new(std::nothrow) OuiWindow_t();
    if (!win) {
        glXMakeCurrent(display, None, nullptr);
        glXDestroyContext(display, glx_ctx);
        XDestroyWindow(display, window);
        XFreeColormap(display, colormap);
        XCloseDisplay(display);
        return nullptr;
    }
    win->display = display;
    win->window = window;
    win->glx_context = glx_ctx;
    win->colormap = colormap;
    win->wm_delete_message = wm_delete;
    win->width = width;
    win->height = height;
    win->backend = backend;
    win->canvas_valid = false;
    win->has_pending_text_input = false;

    // Open X Input Method for proper UTF-8 text input
    win->xim = XOpenIM(display, nullptr, nullptr, nullptr);
    win->xic = nullptr;
    if (win->xim) {
        win->xic = XCreateIC(win->xim,
            XNInputStyle, XIMPreeditNothing | XIMStatusNothing,
            XNClientWindow, window,
            XNFocusWindow, window,
            nullptr);
    }

    // DPI detection — guard against DisplayWidthMM returning 0 (VMs, headless)
    int width_mm = DisplayWidthMM(display, screen);
    if (width_mm > 0) {
        int dpi_x = DisplayWidth(display, screen) * 254 / (width_mm * 10);
        win->dpi_scale = dpi_x > 0 ? static_cast<float>(dpi_x) / 96.0f : 1.0f;
    } else {
        win->dpi_scale = 1.0f;
    }

    // Create Skia GPU context
#ifdef SK_GL
    sk_sp<const GrGLInterface> gl_interface = GrGLInterfaces::MakeGLX();
    if (gl_interface) {
        win->gr_context = GrDirectContexts::MakeGL(gl_interface);
    }

    if (win->gr_context) {
        recreate_surface(win);
    }
#endif

    if (!win->canvas_valid) {
        fprintf(stderr, "oui_window: Warning — GPU surface not created, "
                "use oui_sk_surface_create_raster for CPU rendering\n");
    }

    return win;
}

void oui_window_destroy(OuiWindow window) {
    if (!window) return;

#ifdef SK_GL
    window->surface.reset();
    if (window->gr_context) {
        window->gr_context->abandonContext();
        window->gr_context.reset();
    }
#endif

    if (window->glx_context) {
        glXMakeCurrent(window->display, None, nullptr);
        glXDestroyContext(window->display, window->glx_context);
    }

    if (window->xic) XDestroyIC(window->xic);
    if (window->xim) XCloseIM(window->xim);
    XDestroyWindow(window->display, window->window);
    XFreeColormap(window->display, window->colormap);
    XCloseDisplay(window->display);
    delete window;
}

OuiSkCanvas oui_window_get_canvas(OuiWindow window) {
    if (!window || !window->canvas_valid) return nullptr;
    return &window->canvas_handle;
}

void oui_window_present(OuiWindow window) {
    if (!window) return;

#ifdef SK_GL
    if (window->gr_context) {
        window->gr_context->flushAndSubmit(GrSyncCpu::kNo);
    }
#endif

    glXSwapBuffers(window->display, window->window);
}

bool oui_window_poll_event(OuiWindow window, OuiEvent* event) {
    if (!window || !event) return false;

    memset(event, 0, sizeof(OuiEvent));

    // Drain any pending text input event first
    if (window->has_pending_text_input) {
        *event = window->pending_text_event;
        window->has_pending_text_input = false;
        return true;
    }

    while (XPending(window->display)) {
        XEvent xev;
        XNextEvent(window->display, &xev);

        // Let XIM consume compose/IME events before dispatching
        if (XFilterEvent(&xev, None)) continue;

        switch (xev.type) {
            case FocusIn:
                if (window->xic) XSetICFocus(window->xic);
                break;
            case FocusOut:
                if (window->xic) XUnsetICFocus(window->xic);
                break;

            case Expose:
                event->type = OUI_EVENT_WINDOW_EXPOSED;
                return true;

            case ConfigureNotify: {
                int new_w = xev.xconfigure.width;
                int new_h = xev.xconfigure.height;
                if (new_w != window->width || new_h != window->height) {
                    window->width = new_w;
                    window->height = new_h;
                    recreate_surface(window);
                    event->type = OUI_EVENT_WINDOW_RESIZED;
                    event->resize.width = new_w;
                    event->resize.height = new_h;
                    return true;
                }
                break;
            }

            case MotionNotify:
                event->type = OUI_EVENT_MOUSE_MOTION;
                event->mouse_motion.x = static_cast<float>(xev.xmotion.x);
                event->mouse_motion.y = static_cast<float>(xev.xmotion.y);
                return true;

            case ButtonPress:
                if (xev.xbutton.button == 4 || xev.xbutton.button == 5) {
                    event->type = OUI_EVENT_MOUSE_WHEEL;
                    event->mouse_wheel.dx = 0;
                    event->mouse_wheel.dy = (xev.xbutton.button == 4) ? 1.0f : -1.0f;
                } else {
                    event->type = OUI_EVENT_MOUSE_BUTTON_DOWN;
                    event->mouse_button.x = static_cast<float>(xev.xbutton.x);
                    event->mouse_button.y = static_cast<float>(xev.xbutton.y);
                    event->mouse_button.button = xev.xbutton.button;
                }
                return true;

            case ButtonRelease:
                if (xev.xbutton.button != 4 && xev.xbutton.button != 5) {
                    event->type = OUI_EVENT_MOUSE_BUTTON_UP;
                    event->mouse_button.x = static_cast<float>(xev.xbutton.x);
                    event->mouse_button.y = static_cast<float>(xev.xbutton.y);
                    event->mouse_button.button = xev.xbutton.button;
                    return true;
                }
                break;

            case KeyPress:
            case KeyRelease: {
                event->type = (xev.type == KeyPress) ?
                    OUI_EVENT_KEY_DOWN : OUI_EVENT_KEY_UP;
                event->key.keycode = XLookupKeysym(&xev.xkey, 0);
                event->key.scancode = xev.xkey.keycode;
                event->key.repeat = false;

                // Try to get UTF-8 text input for KeyPress
                if (xev.type == KeyPress) {
                    char buf[32] = {};
                    char* text_buf = buf;
                    KeySym keysym;
                    Status status = XLookupNone;
                    int len = 0;

                    if (window->xic) {
                        // Use Xutf8LookupString for proper UTF-8 + IME support
                        len = Xutf8LookupString(window->xic, &xev.xkey,
                                                buf, sizeof(buf) - 1,
                                                &keysym, &status);
                        // Handle XBufferOverflow: retry with a larger heap buffer
                        if (status == XBufferOverflow && len > 0) {
                            char* heap_buf = static_cast<char*>(
                                malloc(static_cast<size_t>(len) + 1));
                            if (heap_buf) {
                                memset(heap_buf, 0, static_cast<size_t>(len) + 1);
                                len = Xutf8LookupString(window->xic, &xev.xkey,
                                                        heap_buf, len,
                                                        &keysym, &status);
                                text_buf = heap_buf;
                            }
                        }
                    } else {
                        // Fallback to XLookupString (locale-encoded, no IME)
                        len = XLookupString(&xev.xkey, buf, sizeof(buf) - 1,
                                            &keysym, nullptr);
                    }

                    if (len > 0 && static_cast<unsigned char>(text_buf[0]) >= 32) {
                        // Truncate to fit text_input.text (32 bytes with null),
                        // ensuring we don't split a multibyte UTF-8 sequence
                        int copy_len = len < 31 ? len : 31;
                        if (copy_len < len) {
                            // Find the start of the last codepoint in the copy range
                            int i = copy_len - 1;
                            while (i >= 0 &&
                                   (static_cast<unsigned char>(text_buf[i]) & 0xC0) == 0x80) {
                                i--;
                            }
                            // i is the lead byte of the last codepoint; check if it fits
                            if (i >= 0) {
                                unsigned char lead = static_cast<unsigned char>(text_buf[i]);
                                int seq_len = 1;
                                if ((lead & 0xE0) == 0xC0) seq_len = 2;
                                else if ((lead & 0xF0) == 0xE0) seq_len = 3;
                                else if ((lead & 0xF8) == 0xF0) seq_len = 4;
                                if (i + seq_len > copy_len) {
                                    copy_len = i;  // incomplete codepoint, exclude it
                                }
                            }
                        }

                        if (copy_len > 0) {
                            window->has_pending_text_input = true;
                            memset(&window->pending_text_event, 0, sizeof(OuiEvent));
                            window->pending_text_event.type = OUI_EVENT_TEXT_INPUT;
                            memcpy(window->pending_text_event.text_input.text,
                                   text_buf, copy_len);
                        }
                    }

                    if (text_buf != buf) free(text_buf);
                }
                return true;
            }

            case ClientMessage:
                if (static_cast<Atom>(xev.xclient.data.l[0]) == window->wm_delete_message) {
                    event->type = OUI_EVENT_QUIT;
                    return true;
                }
                break;
        }
    }

    return false;
}

OuiSkSize oui_window_get_size(OuiWindow window) {
    OuiSkSize size = {0, 0};
    if (!window) return size;
    size.width = static_cast<float>(window->width);
    size.height = static_cast<float>(window->height);
    return size;
}

float oui_window_get_dpi_scale(OuiWindow window) {
    if (!window) return 1.0f;
    return window->dpi_scale;
}

}  // extern "C"
