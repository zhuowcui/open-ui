//! Render all 10 web apps using Open UI's view! macro framework.
//!
//! Each app uses `inject_css()` with the exact same CSS from the HTML file
//! and `view!{}` macro to build the same DOM structure. This validates that
//! the view! macro framework produces identical output to `load_html()`.

use openui::prelude::*;
use std::path::PathBuf;

fn find_project_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        if dir.join("BUILD.gn").exists() && dir.join("src").is_dir() {
            return dir;
        }
        if !dir.pop() {
            panic!("Cannot find project root (directory containing BUILD.gn)");
        }
    }
}

fn render_app(name: &str, css: &str, view_fn: impl FnOnce() -> ViewNode, out: &str) {
    let mut app = App::new(1200, 800);
    app.inject_css(css);
    app.render(view_fn);
    app.run_frames(1);
    let path = format!("{}/{}.png", out, name);
    app.render_to_png(&path);
    println!("  Framework: {}", name);
}

fn render_01_landing_page(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #ffffff; }

.hero {
  background: linear-gradient(135deg, #2b6cb0, #4299e1);
  color: #ffffff;
  padding: 80px 40px;
  text-align: center;
}
.hero h1 { font-size: 48px; font-weight: 700; margin-bottom: 16px; }
.hero p { font-size: 20px; opacity: 0.9; margin-bottom: 32px; max-width: 600px; margin-left: auto; margin-right: auto; line-height: 1.6; }
.hero .cta {
  display: inline-block;
  background: #ffffff;
  color: #2b6cb0;
  font-size: 18px;
  font-weight: 600;
  padding: 14px 40px;
  border-radius: 8px;
  text-decoration: none;
  border: none;
  cursor: pointer;
}

.features {
  padding: 60px 40px;
  background: #f7fafc;
}
.features h2 { text-align: center; font-size: 32px; margin-bottom: 40px; color: #1a202c; }
.features-grid {
  display: flex;
  gap: 32px;
  max-width: 1000px;
  margin: 0 auto;
  justify-content: center;
}
.feature-card {
  background: #ffffff;
  border-radius: 12px;
  padding: 32px 24px;
  flex: 1;
  text-align: center;
  box-shadow: 0 2px 8px rgba(0,0,0,0.08);
  border: 1px solid #e2e8f0;
}
.feature-card .icon { font-size: 40px; margin-bottom: 16px; }
.feature-card h3 { font-size: 20px; margin-bottom: 12px; color: #1a202c; }
.feature-card p { font-size: 15px; color: #718096; line-height: 1.6; }

footer {
  background: #1a202c;
  color: #a0aec0;
  padding: 40px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
footer .brand { font-size: 18px; font-weight: 600; color: #ffffff; }
footer .links { display: flex; gap: 24px; }
footer .links a { color: #a0aec0; text-decoration: none; font-size: 14px; }
    "##;

    render_app("01-landing-page", css, || view! {
        <section class="hero">
            <h1>
                "Build Better Products"
            </h1>
            <p>
                "The modern platform for teams who want to ship faster, collaborate better, and deliver exceptional user experiences."
            </p>
            <button class="cta">
                "Get Started Free"
            </button>
        </section>
        <section class="features">
            <h2>
                "Why Teams Love Us"
            </h2>
            <div class="features-grid">
                <div class="feature-card">
                    <div class="icon">
                        "⚡"
                    </div>
                    <h3>
                        "Lightning Fast"
                    </h3>
                    <p>
                        "Optimized for speed with sub-second load times and instant interactions across all devices."
                    </p>
                </div>
                <div class="feature-card">
                    <div class="icon">
                        "🔒"
                    </div>
                    <h3>
                        "Secure by Default"
                    </h3>
                    <p>
                        "Enterprise-grade security with end-to-end encryption and compliance certifications built in."
                    </p>
                </div>
                <div class="feature-card">
                    <div class="icon">
                        "📊"
                    </div>
                    <h3>
                        "Rich Analytics"
                    </h3>
                    <p>
                        "Gain deep insights with real-time dashboards, custom reports, and predictive analytics."
                    </p>
                </div>
            </div>
        </section>
        <footer>
            <div class="brand">
                "OpenUI"
            </div>
            <div class="links">
                <a href="#">
                    "About"
                </a>
                <a href="#">
                    "Documentation"
                </a>
                <a href="#">
                    "Blog"
                </a>
                <a href="#">
                    "Contact"
                </a>
                <a href="#">
                    "Privacy"
                </a>
            </div>
        </footer>
    }, out);
}

fn render_02_pricing_table(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #f7fafc; }

.container {
  max-width: 1100px;
  margin: 0 auto;
  padding: 60px 40px;
}
h1 { text-align: center; font-size: 36px; color: #1a202c; margin-bottom: 12px; }
.subtitle { text-align: center; font-size: 18px; color: #718096; margin-bottom: 48px; }

.pricing-grid {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}
.plan {
  flex: 1;
  background: #ffffff;
  border-radius: 12px;
  padding: 36px 28px;
  border: 2px solid #e2e8f0;
  text-align: center;
}
.plan.featured {
  border-color: #4299e1;
  box-shadow: 0 8px 24px rgba(66, 153, 225, 0.2);
  position: relative;
  transform: scale(1.02);
}
.plan .badge {
  display: none;
}
.plan.featured .badge {
  display: inline-block;
  background: #4299e1;
  color: #ffffff;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 16px;
  border-radius: 20px;
  margin-bottom: 16px;
  text-transform: uppercase;
  letter-spacing: 1px;
}
.plan h2 { font-size: 22px; color: #2d3748; margin-bottom: 8px; }
.plan .price { font-size: 48px; font-weight: 700; color: #1a202c; margin: 16px 0 4px; }
.plan .price span { font-size: 16px; font-weight: 400; color: #718096; }
.plan .period { font-size: 14px; color: #a0aec0; margin-bottom: 24px; }
.plan .divider { height: 1px; background: #e2e8f0; margin: 20px 0; }
.plan ul { list-style: none; text-align: left; margin-bottom: 28px; }
.plan ul li { padding: 8px 0; font-size: 15px; color: #4a5568; }
.plan ul li::before { content: "✓"; color: #48bb78; font-weight: 700; margin-right: 10px; }
.plan .btn {
  display: block;
  width: 100%;
  padding: 12px;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  border: 2px solid #e2e8f0;
  background: #ffffff;
  color: #4a5568;
  cursor: pointer;
  text-decoration: none;
  text-align: center;
}
.plan.featured .btn {
  background: #4299e1;
  border-color: #4299e1;
  color: #ffffff;
}
    "##;

    render_app("02-pricing-table", css, || view! {
        <div class="container">
            <h1>
                "Simple, Transparent Pricing"
            </h1>
            <p class="subtitle">
                "Choose the plan that fits your needs. Upgrade or downgrade anytime."
            </p>
            <div class="pricing-grid">
                <div class="plan">
                    <div class="badge">
                        "Most Popular"
                    </div>
                    <h2>
                        "Basic"
                    </h2>
                    <div class="price">
                        "$9"
                        <span>
                            "/mo"
                        </span>
                    </div>
                    <div class="period">
                        "Billed monthly"
                    </div>
                    <div class="divider">
                    </div>
                    <ul>
                        <li>
                            "5 Projects"
                        </li>
                        <li>
                            "10 GB Storage"
                        </li>
                        <li>
                            "Basic Analytics"
                        </li>
                        <li>
                            "Email Support"
                        </li>
                    </ul>
                    <a class="btn" href="#">
                        "Get Started"
                    </a>
                </div>
                <div class="plan featured">
                    <div class="badge">
                        "Most Popular"
                    </div>
                    <h2>
                        "Pro"
                    </h2>
                    <div class="price">
                        "$29"
                        <span>
                            "/mo"
                        </span>
                    </div>
                    <div class="period">
                        "Billed monthly"
                    </div>
                    <div class="divider">
                    </div>
                    <ul>
                        <li>
                            "Unlimited Projects"
                        </li>
                        <li>
                            "100 GB Storage"
                        </li>
                        <li>
                            "Advanced Analytics"
                        </li>
                        <li>
                            "Priority Support"
                        </li>
                        <li>
                            "Custom Domains"
                        </li>
                        <li>
                            "Team Collaboration"
                        </li>
                    </ul>
                    <a class="btn" href="#">
                        "Get Started"
                    </a>
                </div>
                <div class="plan">
                    <div class="badge">
                        "Most Popular"
                    </div>
                    <h2>
                        "Enterprise"
                    </h2>
                    <div class="price">
                        "$99"
                        <span>
                            "/mo"
                        </span>
                    </div>
                    <div class="period">
                        "Billed monthly"
                    </div>
                    <div class="divider">
                    </div>
                    <ul>
                        <li>
                            "Unlimited Everything"
                        </li>
                        <li>
                            "1 TB Storage"
                        </li>
                        <li>
                            "Real-time Analytics"
                        </li>
                        <li>
                            "24/7 Phone Support"
                        </li>
                        <li>
                            "SLA Guarantee"
                        </li>
                        <li>
                            "SSO & SAML"
                        </li>
                        <li>
                            "Dedicated Manager"
                        </li>
                    </ul>
                    <a class="btn" href="#">
                        "Contact Sales"
                    </a>
                </div>
            </div>
        </div>
    }, out);
}

fn render_03_login_form(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: #2d3748;
  background: #edf2f7;
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 800px;
}

.card {
  background: #ffffff;
  border-radius: 12px;
  padding: 48px 40px;
  width: 420px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.08);
}

.logo {
  text-align: center;
  margin-bottom: 32px;
}
.logo .icon {
  width: 56px;
  height: 56px;
  background: #4299e1;
  border-radius: 12px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: #ffffff;
  margin-bottom: 16px;
}
.logo h1 { font-size: 24px; color: #1a202c; }
.logo p { font-size: 14px; color: #718096; margin-top: 4px; }

.form-group {
  margin-bottom: 20px;
}
.form-group label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: #4a5568;
  margin-bottom: 6px;
}
.form-group input[type="text"],
.form-group input[type="password"] {
  width: 100%;
  padding: 10px 14px;
  border: 2px solid #e2e8f0;
  border-radius: 8px;
  font-size: 15px;
  font-family: inherit;
  color: #2d3748;
  background: #ffffff;
  outline: none;
}

.options {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.remember {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #4a5568;
}
.remember input[type="checkbox"] {
  width: 16px;
  height: 16px;
}
.forgot {
  font-size: 14px;
  color: #4299e1;
  text-decoration: none;
  font-weight: 500;
}

.login-btn {
  width: 100%;
  padding: 12px;
  background: #4299e1;
  color: #ffffff;
  border: none;
  border-radius: 8px;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
  font-family: inherit;
  margin-bottom: 20px;
}

.divider {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 20px;
}
.divider .line { flex: 1; height: 1px; background: #e2e8f0; }
.divider span { font-size: 13px; color: #a0aec0; }

.signup {
  text-align: center;
  font-size: 14px;
  color: #718096;
}
.signup a { color: #4299e1; text-decoration: none; font-weight: 600; }
    "##;

    render_app("03-login-form", css, || view! {
        <div class="card">
            <div class="logo">
                <div class="icon">
                    "🔑"
                </div>
                <h1>
                    "Welcome Back"
                </h1>
                <p>
                    "Sign in to your account"
                </p>
            </div>
            <div class="form-group">
                <label>
                    "Email Address"
                </label>
                <input type="text" placeholder="you@example.com" />
            </div>
            <div class="form-group">
                <label>
                    "Password"
                </label>
                <input type="password" placeholder="Enter your password" />
            </div>
            <div class="options">
                <label class="remember">
                    <input type="checkbox" />
                    "Remember me"
                </label>
                <a class="forgot" href="#">
                    "Forgot password?"
                </a>
            </div>
            <button class="login-btn">
                "Sign In"
            </button>
            <div class="divider">
                <div class="line">
                </div>
                <span>
                    "or"
                </span>
                <div class="line">
                </div>
            </div>
            <p class="signup">
                "Don't have an account?"
                <a href="#">
                    "Sign up"
                </a>
            </p>
        </div>
    }, out);
}

fn render_04_profile_card(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  color: #2d3748;
  background: #edf2f7;
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 800px;
}

.card {
  background: #ffffff;
  border-radius: 16px;
  width: 380px;
  overflow: hidden;
  box-shadow: 0 4px 16px rgba(0,0,0,0.1);
}

.banner {
  height: 100px;
  background: linear-gradient(135deg, #4299e1, #2b6cb0);
}

.profile-section {
  padding: 0 28px 28px;
  text-align: center;
}

.avatar {
  width: 88px;
  height: 88px;
  background: #3182ce;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 32px;
  font-weight: 700;
  color: #ffffff;
  border: 4px solid #ffffff;
  margin-top: -44px;
}

.name { font-size: 22px; font-weight: 700; color: #1a202c; margin-top: 12px; }
.role { font-size: 14px; color: #4299e1; font-weight: 500; margin-top: 4px; }
.bio { font-size: 14px; color: #718096; line-height: 1.6; margin-top: 12px; padding: 0 8px; }

.stats {
  display: flex;
  justify-content: center;
  gap: 32px;
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid #e2e8f0;
}
.stat { text-align: center; }
.stat .number { font-size: 20px; font-weight: 700; color: #1a202c; }
.stat .label { font-size: 12px; color: #a0aec0; text-transform: uppercase; letter-spacing: 0.5px; margin-top: 2px; }

.actions {
  display: flex;
  gap: 12px;
  margin-top: 24px;
  padding: 0 8px;
}
.btn {
  flex: 1;
  padding: 10px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  font-family: inherit;
  text-align: center;
}
.btn-primary { background: #4299e1; color: #ffffff; }
.btn-secondary { background: #edf2f7; color: #4a5568; }

.social {
  display: flex;
  justify-content: center;
  gap: 16px;
  margin-top: 20px;
  padding-top: 20px;
  border-top: 1px solid #e2e8f0;
}
.social a {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: #f7fafc;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 16px;
  text-decoration: none;
  border: 1px solid #e2e8f0;
}
    "##;

    render_app("04-profile-card", css, || view! {
        <div class="card">
            <div class="banner">
            </div>
            <div class="profile-section">
                <div class="avatar">
                    "JD"
                </div>
                <div class="name">
                    "Jane Doe"
                </div>
                <div class="role">
                    "Senior Product Designer"
                </div>
                <p class="bio">
                    "Passionate about creating beautiful and intuitive user experiences. Previously at Google and Figma."
                </p>
                <div class="stats">
                    <div class="stat">
                        <div class="number">
                            "284"
                        </div>
                        <div class="label">
                            "Posts"
                        </div>
                    </div>
                    <div class="stat">
                        <div class="number">
                            "14.2k"
                        </div>
                        <div class="label">
                            "Followers"
                        </div>
                    </div>
                    <div class="stat">
                        <div class="number">
                            "892"
                        </div>
                        <div class="label">
                            "Following"
                        </div>
                    </div>
                </div>
                <div class="actions">
                    <button class="btn btn-primary">
                        "Follow"
                    </button>
                    <button class="btn btn-secondary">
                        "Message"
                    </button>
                </div>
                <div class="social">
                    <a href="#">
                        "🐦"
                    </a>
                    <a href="#">
                        "💼"
                    </a>
                    <a href="#">
                        "🌐"
                    </a>
                    <a href="#">
                        "📷"
                    </a>
                </div>
            </div>
        </div>
    }, out);
}

fn render_05_navigation_bar(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #ffffff; }

nav {
  background: #ffffff;
  border-bottom: 1px solid #e2e8f0;
  padding: 0 40px;
  display: flex;
  align-items: center;
  height: 64px;
  position: sticky;
  top: 0;
  z-index: 100;
}
.nav-brand {
  font-size: 20px;
  font-weight: 700;
  color: #2b6cb0;
  margin-right: 48px;
}
.nav-links {
  display: flex;
  gap: 4px;
  flex: 1;
}
.nav-links a {
  text-decoration: none;
  font-size: 15px;
  font-weight: 500;
  color: #718096;
  padding: 8px 16px;
  border-radius: 6px;
}
.nav-links a.active {
  color: #2b6cb0;
  background: #ebf8ff;
}
.nav-cta {
  background: #4299e1;
  color: #ffffff;
  padding: 8px 20px;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  font-family: inherit;
}

.breadcrumb {
  padding: 16px 40px;
  background: #f7fafc;
  border-bottom: 1px solid #e2e8f0;
  font-size: 14px;
  color: #a0aec0;
  display: flex;
  gap: 8px;
  align-items: center;
}
.breadcrumb a { color: #4299e1; text-decoration: none; }
.breadcrumb span { color: #cbd5e0; }

.content {
  max-width: 900px;
  margin: 0 auto;
  padding: 40px;
}
.content h1 { font-size: 32px; color: #1a202c; margin-bottom: 8px; }
.content .desc { font-size: 16px; color: #718096; margin-bottom: 32px; line-height: 1.6; }

.section {
  margin-bottom: 32px;
}
.section h2 { font-size: 22px; color: #1a202c; margin-bottom: 12px; padding-bottom: 8px; border-bottom: 2px solid #e2e8f0; }
.section p { font-size: 15px; color: #4a5568; line-height: 1.7; margin-bottom: 12px; }

.card-row {
  display: flex;
  gap: 20px;
  margin-top: 20px;
}
.info-card {
  flex: 1;
  padding: 20px;
  background: #f7fafc;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}
.info-card h3 { font-size: 16px; color: #2d3748; margin-bottom: 8px; }
.info-card p { font-size: 14px; color: #718096; line-height: 1.5; }
    "##;

    render_app("05-navigation-bar", css, || view! {
        <nav>
            <div class="nav-brand">
                "OpenUI"
            </div>
            <div class="nav-links">
                <a href="#">
                    "Home"
                </a>
                <a href="#" class="active">
                    "Products"
                </a>
                <a href="#">
                    "Solutions"
                </a>
                <a href="#">
                    "Docs"
                </a>
                <a href="#">
                    "Pricing"
                </a>
            </div>
            <button class="nav-cta">
                "Sign Up"
            </button>
        </nav>
        <div class="breadcrumb">
            <a href="#">
                "Home"
            </a>
            <span>
                "/"
            </span>
            <a href="#">
                "Products"
            </a>
            <span>
                "/"
            </span>
            <span style:color="#4a5568">
                "UI Framework"
            </span>
        </div>
        <div class="content">
            <h1>
                "UI Framework"
            </h1>
            <p class="desc">
                "A comprehensive design system and component library for building modern web applications with consistency and speed."
            </p>
            <div class="section">
                <h2>
                    "Overview"
                </h2>
                <p>
                    "Our UI Framework provides a complete set of pre-built components, design tokens, and layout utilities. Build production-ready interfaces in hours, not weeks."
                </p>
                <p>
                    "With support for dark mode, responsive layouts, and accessibility out of the box, your applications will meet the highest standards from day one."
                </p>
            </div>
            <div class="card-row">
                <div class="info-card">
                    <h3>
                        "🧩 Components"
                    </h3>
                    <p>
                        "50+ ready-to-use components from buttons to complex data tables."
                    </p>
                </div>
                <div class="info-card">
                    <h3>
                        "🎨 Theming"
                    </h3>
                    <p>
                        "Fully customizable design tokens for colors, typography, and spacing."
                    </p>
                </div>
                <div class="info-card">
                    <h3>
                        "♿ Accessible"
                    </h3>
                    <p>
                        "WCAG 2.1 AA compliant with full keyboard navigation support."
                    </p>
                </div>
            </div>
        </div>
    }, out);
}

fn render_06_data_table(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #f7fafc; }

.container {
  max-width: 1100px;
  margin: 0 auto;
  padding: 48px 40px;
}

.header {
  margin-bottom: 24px;
}
.header h1 { font-size: 28px; color: #1a202c; margin-bottom: 4px; }
.header p { font-size: 15px; color: #718096; }

.table-card {
  background: #ffffff;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 2px 8px rgba(0,0,0,0.06);
  border: 1px solid #e2e8f0;
}

table {
  width: 100%;
  border-collapse: collapse;
}
thead { background: #f7fafc; }
th {
  text-align: left;
  padding: 14px 20px;
  font-size: 12px;
  font-weight: 600;
  color: #718096;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-bottom: 2px solid #e2e8f0;
}
td {
  padding: 14px 20px;
  font-size: 14px;
  border-bottom: 1px solid #edf2f7;
}
tr:nth-child(even) { background: #f7fafc; }
tr:last-child td { border-bottom: none; }

.name-cell {
  display: flex;
  align-items: center;
  gap: 12px;
}
.name-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  color: #ffffff;
}
.name-info .full-name { font-weight: 600; color: #1a202c; }
.name-info .username { font-size: 13px; color: #a0aec0; }

.badge {
  display: inline-block;
  padding: 3px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 600;
}
.badge-active { background: #c6f6d5; color: #276749; }
.badge-inactive { background: #fed7d7; color: #9b2c2c; }
.badge-pending { background: #fefcbf; color: #975a16; }

.role { color: #4a5568; }
.email { color: #718096; }

.table-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-top: 1px solid #e2e8f0;
  font-size: 14px;
  color: #718096;
}
    "##;

    render_app("06-data-table", css, || view! {
        <div class="container">
            <div class="header">
                <h1>
                    "Team Members"
                </h1>
                <p>
                    "Manage your team members and their account permissions here."
                </p>
            </div>
            <div class="table-card">
                <table>
                    <thead>
                        <tr>
                            <th>
                                "Name"
                            </th>
                            <th>
                                "Email"
                            </th>
                            <th>
                                "Role"
                            </th>
                            <th>
                                "Status"
                            </th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#4299e1">
                                        "AL"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Alice Lee"
                                        </div>
                                        <div class="username">
                                            "@alicelee"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "alice.lee@company.com"
                            </td>
                            <td class="role">
                                "Admin"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#48bb78">
                                        "BW"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Bob Wilson"
                                        </div>
                                        <div class="username">
                                            "@bobwilson"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "bob.wilson@company.com"
                            </td>
                            <td class="role">
                                "Developer"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#ed8936">
                                        "CJ"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Carol Johnson"
                                        </div>
                                        <div class="username">
                                            "@carolj"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "carol.j@company.com"
                            </td>
                            <td class="role">
                                "Designer"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#9f7aea">
                                        "DM"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "David Martinez"
                                        </div>
                                        <div class="username">
                                            "@davidm"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "david.m@company.com"
                            </td>
                            <td class="role">
                                "Developer"
                            </td>
                            <td>
                                <span class="badge badge-inactive">
                                    "Inactive"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#f56565">
                                        "EB"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Eve Brown"
                                        </div>
                                        <div class="username">
                                            "@evebrown"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "eve.brown@company.com"
                            </td>
                            <td class="role">
                                "Product Manager"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#38b2ac">
                                        "FK"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Frank Kim"
                                        </div>
                                        <div class="username">
                                            "@frankk"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "frank.kim@company.com"
                            </td>
                            <td class="role">
                                "DevOps"
                            </td>
                            <td>
                                <span class="badge badge-pending">
                                    "Pending"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#e53e3e">
                                        "GD"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Grace Davis"
                                        </div>
                                        <div class="username">
                                            "@graced"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "grace.d@company.com"
                            </td>
                            <td class="role">
                                "QA Engineer"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#3182ce">
                                        "HT"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Henry Taylor"
                                        </div>
                                        <div class="username">
                                            "@henryt"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "henry.t@company.com"
                            </td>
                            <td class="role">
                                "Intern"
                            </td>
                            <td>
                                <span class="badge badge-active">
                                    "Active"
                                </span>
                            </td>
                        </tr>
                        <tr>
                            <td>
                                <div class="name-cell">
                                    <div class="name-avatar" style:background="#d69e2e">
                                        "IN"
                                    </div>
                                    <div class="name-info">
                                        <div class="full-name">
                                            "Iris Nguyen"
                                        </div>
                                        <div class="username">
                                            "@irisn"
                                        </div>
                                    </div>
                                </div>
                            </td>
                            <td class="email">
                                "iris.n@company.com"
                            </td>
                            <td class="role">
                                "Designer"
                            </td>
                            <td>
                                <span class="badge badge-inactive">
                                    "Inactive"
                                </span>
                            </td>
                        </tr>
                    </tbody>
                </table>
                <div class="table-footer">
                    <span>
                        "Showing 1-9 of 24 members"
                    </span>
                    <span>
                        "Page 1 of 3"
                    </span>
                </div>
            </div>
        </div>
    }, out);
}

fn render_07_dashboard_stats(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #f7fafc; }

.dashboard {
  padding: 32px 40px;
  max-width: 1200px;
  margin: 0 auto;
}

.dash-header {
  margin-bottom: 28px;
}
.dash-header h1 { font-size: 28px; color: #1a202c; }
.dash-header p { font-size: 15px; color: #718096; margin-top: 4px; }

.metrics {
  display: flex;
  gap: 20px;
  margin-bottom: 28px;
}
.metric-card {
  flex: 1;
  background: #ffffff;
  border-radius: 12px;
  padding: 24px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 4px rgba(0,0,0,0.04);
}
.metric-card .label { font-size: 13px; color: #718096; font-weight: 500; text-transform: uppercase; letter-spacing: 0.5px; }
.metric-card .value { font-size: 32px; font-weight: 700; color: #1a202c; margin-top: 8px; }
.metric-card .change { font-size: 13px; margin-top: 8px; font-weight: 500; }
.metric-card .change.up { color: #38a169; }
.metric-card .change.down { color: #e53e3e; }
.metric-card .icon { font-size: 24px; float: right; }

.row {
  display: flex;
  gap: 20px;
}

.chart-card {
  flex: 2;
  background: #ffffff;
  border-radius: 12px;
  padding: 24px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 4px rgba(0,0,0,0.04);
}
.chart-card h2 { font-size: 18px; color: #1a202c; margin-bottom: 20px; }

.chart {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  height: 180px;
  padding-top: 8px;
}
.bar-group {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  height: 100%;
  justify-content: flex-end;
}
.bar {
  width: 100%;
  border-radius: 4px 4px 0 0;
  min-height: 8px;
}
.bar-label {
  font-size: 12px;
  color: #a0aec0;
  margin-top: 8px;
}

.activity-card {
  flex: 1;
  background: #ffffff;
  border-radius: 12px;
  padding: 24px;
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 4px rgba(0,0,0,0.04);
}
.activity-card h2 { font-size: 18px; color: #1a202c; margin-bottom: 16px; }

.activity-item {
  display: flex;
  gap: 12px;
  padding: 12px 0;
  border-bottom: 1px solid #edf2f7;
  align-items: flex-start;
}
.activity-item:last-child { border-bottom: none; }
.activity-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 6px;
  flex-shrink: 0;
}
.activity-text { font-size: 14px; color: #4a5568; line-height: 1.4; }
.activity-text strong { color: #1a202c; }
.activity-time { font-size: 12px; color: #a0aec0; margin-top: 2px; }
    "##;

    render_app("07-dashboard-stats", css, || view! {
        <div class="dashboard">
            <div class="dash-header">
                <h1>
                    "Dashboard"
                </h1>
                <p>
                    "Overview of your business metrics and recent activity."
                </p>
            </div>
            <div class="metrics">
                <div class="metric-card">
                    <span class="icon">
                        "👥"
                    </span>
                    <div class="label">
                        "Total Users"
                    </div>
                    <div class="value">
                        "12,847"
                    </div>
                    <div class="change up">
                        "↑ 12.5% from last month"
                    </div>
                </div>
                <div class="metric-card">
                    <span class="icon">
                        "💰"
                    </span>
                    <div class="label">
                        "Revenue"
                    </div>
                    <div class="value">
                        "$48,392"
                    </div>
                    <div class="change up">
                        "↑ 8.2% from last month"
                    </div>
                </div>
                <div class="metric-card">
                    <span class="icon">
                        "📦"
                    </span>
                    <div class="label">
                        "Orders"
                    </div>
                    <div class="value">
                        "1,429"
                    </div>
                    <div class="change down">
                        "↓ 3.1% from last month"
                    </div>
                </div>
                <div class="metric-card">
                    <span class="icon">
                        "📈"
                    </span>
                    <div class="label">
                        "Growth Rate"
                    </div>
                    <div class="value">
                        "23.6%"
                    </div>
                    <div class="change up">
                        "↑ 4.3% from last month"
                    </div>
                </div>
            </div>
            <div class="row">
                <div class="chart-card">
                    <h2>
                        "Monthly Revenue"
                    </h2>
                    <div class="chart">
                        <div class="bar-group">
                            <div class="bar" style:height="45%" style:background="#bee3f8">
                            </div>
                            <div class="bar-label">
                                "Jan"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="55%" style:background="#bee3f8">
                            </div>
                            <div class="bar-label">
                                "Feb"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="40%" style:background="#bee3f8">
                            </div>
                            <div class="bar-label">
                                "Mar"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="65%" style:background="#bee3f8">
                            </div>
                            <div class="bar-label">
                                "Apr"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="52%" style:background="#bee3f8">
                            </div>
                            <div class="bar-label">
                                "May"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="70%" style:background="#4299e1">
                            </div>
                            <div class="bar-label">
                                "Jun"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="85%" style:background="#4299e1">
                            </div>
                            <div class="bar-label">
                                "Jul"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="75%" style:background="#4299e1">
                            </div>
                            <div class="bar-label">
                                "Aug"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="90%" style:background="#4299e1">
                            </div>
                            <div class="bar-label">
                                "Sep"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="80%" style:background="#4299e1">
                            </div>
                            <div class="bar-label">
                                "Oct"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="95%" style:background="#3182ce">
                            </div>
                            <div class="bar-label">
                                "Nov"
                            </div>
                        </div>
                        <div class="bar-group">
                            <div class="bar" style:height="100%" style:background="#2b6cb0">
                            </div>
                            <div class="bar-label">
                                "Dec"
                            </div>
                        </div>
                    </div>
                </div>
                <div class="activity-card">
                    <h2>
                        "Recent Activity"
                    </h2>
                    <div class="activity-item">
                        <div class="activity-dot" style:background="#48bb78">
                        </div>
                        <div>
                            <div class="activity-text">
                                <strong>
                                    "Sarah"
                                </strong>
                                "completed order #1847"
                            </div>
                            <div class="activity-time">
                                "2 minutes ago"
                            </div>
                        </div>
                    </div>
                    <div class="activity-item">
                        <div class="activity-dot" style:background="#4299e1">
                        </div>
                        <div>
                            <div class="activity-text">
                                <strong>
                                    "Mike"
                                </strong>
                                "created a new project"
                            </div>
                            <div class="activity-time">
                                "15 minutes ago"
                            </div>
                        </div>
                    </div>
                    <div class="activity-item">
                        <div class="activity-dot" style:background="#ecc94b">
                        </div>
                        <div>
                            <div class="activity-text">
                                <strong>
                                    "Emily"
                                </strong>
                                "updated billing info"
                            </div>
                            <div class="activity-time">
                                "1 hour ago"
                            </div>
                        </div>
                    </div>
                    <div class="activity-item">
                        <div class="activity-dot" style:background="#f56565">
                        </div>
                        <div>
                            <div class="activity-text">
                                <strong>
                                    "System"
                                </strong>
                                "flagged unusual login"
                            </div>
                            <div class="activity-time">
                                "3 hours ago"
                            </div>
                        </div>
                    </div>
                    <div class="activity-item">
                        <div class="activity-dot" style:background="#48bb78">
                        </div>
                        <div>
                            <div class="activity-text">
                                <strong>
                                    "Alex"
                                </strong>
                                "joined the team"
                            </div>
                            <div class="activity-time">
                                "5 hours ago"
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }, out);
}

fn render_08_blog_post(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #ffffff; }

.article {
  max-width: 720px;
  margin: 0 auto;
  padding: 56px 24px;
}

.article-header {
  margin-bottom: 32px;
}
.article-header .category {
  display: inline-block;
  font-size: 13px;
  font-weight: 600;
  color: #4299e1;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 12px;
}
.article-header h1 {
  font-size: 36px;
  font-weight: 700;
  color: #1a202c;
  line-height: 1.3;
  margin-bottom: 16px;
}
.article-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  color: #718096;
}
.author-avatar {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: #4299e1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  font-weight: 600;
  font-size: 14px;
}
.author-name { font-weight: 600; color: #2d3748; }
.meta-sep { color: #cbd5e0; }

.article-body p {
  font-size: 17px;
  line-height: 1.8;
  color: #4a5568;
  margin-bottom: 20px;
}

.article-body h2 {
  font-size: 24px;
  color: #1a202c;
  margin-top: 36px;
  margin-bottom: 16px;
}

.article-body blockquote {
  border-left: 4px solid #4299e1;
  padding: 16px 24px;
  margin: 24px 0;
  background: #ebf8ff;
  border-radius: 0 8px 8px 0;
  font-size: 17px;
  color: #2b6cb0;
  font-style: italic;
  line-height: 1.7;
}

.article-body pre {
  background: #1a202c;
  color: #e2e8f0;
  padding: 20px 24px;
  border-radius: 8px;
  margin: 24px 0;
  overflow-x: auto;
  font-size: 14px;
  line-height: 1.6;
  font-family: "SF Mono", "Fira Code", monospace;
}
.article-body code {
  font-family: "SF Mono", "Fira Code", monospace;
}
.article-body p code {
  background: #edf2f7;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 15px;
  color: #e53e3e;
}

.article-body ul {
  margin: 16px 0 20px 24px;
}
.article-body ul li {
  font-size: 17px;
  line-height: 1.8;
  color: #4a5568;
  margin-bottom: 6px;
}

.tags {
  display: flex;
  gap: 8px;
  margin-top: 40px;
  padding-top: 24px;
  border-top: 1px solid #e2e8f0;
  flex-wrap: wrap;
}
.tag {
  display: inline-block;
  padding: 4px 14px;
  background: #edf2f7;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 500;
  color: #4a5568;
}
    "##;

    render_app("08-blog-post", css, || view! {
        <article class="article">
            <div class="article-header">
                <div class="category">
                    "Engineering"
                </div>
                <h1>
                    "Building Scalable UI Systems with Modern CSS"
                </h1>
                <div class="article-meta">
                    <div class="author-avatar">
                        "JD"
                    </div>
                    <div>
                        <span class="author-name">
                            "Jane Doe"
                        </span>
                        <span class="meta-sep">
                            "·"
                        </span>
                        <span>
                            "December 15, 2024"
                        </span>
                        <span class="meta-sep">
                            "·"
                        </span>
                        <span>
                            "8 min read"
                        </span>
                    </div>
                </div>
            </div>
            <div class="article-body">
                <p>
                    "Modern CSS has evolved far beyond simple styling. With flexbox, grid, custom properties, and container queries, we now have the tools to build truly scalable design systems that adapt across any context."
                </p>
                <h2>
                    "The Foundation"
                </h2>
                <p>
                    "Every good design system starts with a solid foundation. This means establishing your design tokens early — colors, typography scales, spacing values, and breakpoints that form the vocabulary of your system."
                </p>
                <blockquote>
                    "Design systems are not about enforcing consistency — they are about enabling teams to move faster while maintaining coherence across products."
                </blockquote>
                <p>
                    "Consider using CSS custom properties for your tokens. They cascade naturally, can be overridden in context, and work beautifully with component-scoped styles."
                </p>
                <pre>
                    <code>
                        ":root {
  --color-primary: #4299e1;
  --color-text: #2d3748;
  --space-md: 16px;
  --radius-lg: 12px;
}

.card {
  padding: var(--space-md);
  border-radius: var(--radius-lg);
  color: var(--color-text);
}"
                    </code>
                </pre>
                <h2>
                    "Key Principles"
                </h2>
                <p>
                    "When building your UI system, keep these principles in mind:"
                </p>
                <ul>
                    <li>
                        "Compose small, focused components rather than large monolithic ones"
                    </li>
                    <li>
                        "Use semantic HTML elements as your base building blocks"
                    </li>
                    <li>
                        "Design for content flexibility — components should handle varying content lengths"
                    </li>
                    <li>
                        "Test across viewports from the start, not as an afterthought"
                    </li>
                    <li>
                        "Document usage patterns alongside the component code"
                    </li>
                </ul>
                <p>
                    "The goal is to create a system that is both powerful enough for complex interfaces and simple enough that any team member can contribute to it with confidence."
                </p>
            </div>
            <div class="tags">
                <span class="tag">
                    "CSS"
                </span>
                <span class="tag">
                    "Design Systems"
                </span>
                <span class="tag">
                    "Frontend"
                </span>
                <span class="tag">
                    "Architecture"
                </span>
                <span class="tag">
                    "Web Development"
                </span>
            </div>
        </article>
    }, out);
}

fn render_09_settings_panel(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #f7fafc; }

.layout {
  display: flex;
  min-height: 800px;
}

.sidebar {
  width: 240px;
  background: #ffffff;
  border-right: 1px solid #e2e8f0;
  padding: 24px 0;
  flex-shrink: 0;
}
.sidebar-title {
  font-size: 12px;
  font-weight: 600;
  color: #a0aec0;
  text-transform: uppercase;
  letter-spacing: 1px;
  padding: 0 20px;
  margin-bottom: 12px;
}
.sidebar-nav { list-style: none; }
.sidebar-nav li a {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 20px;
  font-size: 14px;
  color: #718096;
  text-decoration: none;
  font-weight: 500;
}
.sidebar-nav li a.active {
  color: #2b6cb0;
  background: #ebf8ff;
  border-right: 3px solid #4299e1;
  font-weight: 600;
}
.sidebar-nav li a .icon { font-size: 16px; }

.main {
  flex: 1;
  padding: 32px 40px;
  max-width: 800px;
}
.main h1 { font-size: 24px; color: #1a202c; margin-bottom: 4px; }
.main .desc { font-size: 15px; color: #718096; margin-bottom: 32px; }

.section {
  background: #ffffff;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  padding: 24px;
  margin-bottom: 24px;
}
.section h2 { font-size: 18px; color: #1a202c; margin-bottom: 4px; }
.section .section-desc { font-size: 14px; color: #718096; margin-bottom: 20px; }

.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 0;
  border-bottom: 1px solid #edf2f7;
}
.setting-row:last-child { border-bottom: none; }
.setting-info .setting-label { font-size: 15px; font-weight: 500; color: #2d3748; }
.setting-info .setting-hint { font-size: 13px; color: #a0aec0; margin-top: 2px; }

.toggle {
  width: 44px;
  height: 24px;
  border-radius: 12px;
  background: #cbd5e0;
  position: relative;
  cursor: pointer;
}
.toggle.on { background: #4299e1; }
.toggle .knob {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: #ffffff;
  position: absolute;
  top: 2px;
  left: 2px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.15);
}
.toggle.on .knob { left: 22px; }

.form-row {
  margin-bottom: 18px;
}
.form-row label {
  display: block;
  font-size: 14px;
  font-weight: 600;
  color: #4a5568;
  margin-bottom: 6px;
}
.form-row input[type="text"],
.form-row input[type="email"],
.form-row select {
  width: 100%;
  padding: 10px 14px;
  border: 2px solid #e2e8f0;
  border-radius: 8px;
  font-size: 14px;
  font-family: inherit;
  color: #2d3748;
  background: #ffffff;
}
.form-row select { appearance: auto; }

.form-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
  padding-top: 20px;
  border-top: 1px solid #e2e8f0;
  margin-top: 8px;
}
.btn {
  padding: 10px 24px;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  border: none;
  cursor: pointer;
  font-family: inherit;
}
.btn-primary { background: #4299e1; color: #ffffff; }
.btn-cancel { background: #edf2f7; color: #4a5568; }
    "##;

    render_app("09-settings-panel", css, || view! {
        <div class="layout">
            <aside class="sidebar">
                <div class="sidebar-title">
                    "Settings"
                </div>
                <ul class="sidebar-nav">
                    <li>
                        <a href="#" class="active">
                            <span class="icon">
                                "⚙️"
                            </span>
                            "General"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "🔒"
                            </span>
                            "Privacy"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "🔔"
                            </span>
                            "Notifications"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "🎨"
                            </span>
                            "Appearance"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "🔑"
                            </span>
                            "Security"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "💳"
                            </span>
                            "Billing"
                        </a>
                    </li>
                    <li>
                        <a href="#">
                            <span class="icon">
                                "🔗"
                            </span>
                            "Integrations"
                        </a>
                    </li>
                </ul>
            </aside>
            <main class="main">
                <h1>
                    "General Settings"
                </h1>
                <p class="desc">
                    "Manage your account settings and preferences."
                </p>
                <div class="section">
                    <h2>
                        "Profile Information"
                    </h2>
                    <p class="section-desc">
                        "Update your personal details and contact information."
                    </p>
                    <div class="form-row">
                        <label>
                            "Display Name"
                        </label>
                        <input type="text" value="Jane Doe" />
                    </div>
                    <div class="form-row">
                        <label>
                            "Email Address"
                        </label>
                        <input type="email" value="jane.doe@company.com" />
                    </div>
                    <div class="form-row">
                        <label>
                            "Language"
                        </label>
                        <select>
                            <option>
                                "English (US)"
                            </option>
                            <option>
                                "Spanish"
                            </option>
                            <option>
                                "French"
                            </option>
                        </select>
                    </div>
                    <div class="form-row">
                        <label>
                            "Timezone"
                        </label>
                        <select>
                            <option>
                                "Pacific Time (UTC-8)"
                            </option>
                            <option>
                                "Eastern Time (UTC-5)"
                            </option>
                            <option>
                                "UTC"
                            </option>
                        </select>
                    </div>
                </div>
                <div class="section">
                    <h2>
                        "Preferences"
                    </h2>
                    <p class="section-desc">
                        "Customize your experience."
                    </p>
                    <div class="setting-row">
                        <div class="setting-info">
                            <div class="setting-label">
                                "Dark Mode"
                            </div>
                            <div class="setting-hint">
                                "Switch to dark theme for the interface"
                            </div>
                        </div>
                        <div class="toggle">
                            <div class="knob">
                            </div>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div class="setting-info">
                            <div class="setting-label">
                                "Email Notifications"
                            </div>
                            <div class="setting-hint">
                                "Receive email updates about activity"
                            </div>
                        </div>
                        <div class="toggle on">
                            <div class="knob">
                            </div>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div class="setting-info">
                            <div class="setting-label">
                                "Two-Factor Authentication"
                            </div>
                            <div class="setting-hint">
                                "Add extra security to your account"
                            </div>
                        </div>
                        <div class="toggle on">
                            <div class="knob">
                            </div>
                        </div>
                    </div>
                    <div class="setting-row">
                        <div class="setting-info">
                            <div class="setting-label">
                                "Marketing Emails"
                            </div>
                            <div class="setting-hint">
                                "Receive promotional content and updates"
                            </div>
                        </div>
                        <div class="toggle">
                            <div class="knob">
                            </div>
                        </div>
                    </div>
                </div>
                <div class="form-actions">
                    <button class="btn btn-cancel">
                        "Cancel"
                    </button>
                    <button class="btn btn-primary">
                        "Save Changes"
                    </button>
                </div>
            </main>
        </div>
    }, out);
}

fn render_10_kanban_board(out: &str) {
    let css = r##"
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; color: #2d3748; background: #f7fafc; }

.board-header {
  background: #ffffff;
  border-bottom: 1px solid #e2e8f0;
  padding: 20px 32px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.board-header h1 { font-size: 22px; color: #1a202c; }
.board-header .info { font-size: 14px; color: #718096; }

.board {
  display: flex;
  gap: 20px;
  padding: 24px 32px;
  overflow-x: auto;
  min-height: calc(800px - 72px);
  align-items: flex-start;
}

.column {
  min-width: 340px;
  flex: 1;
  background: #edf2f7;
  border-radius: 12px;
  padding: 16px;
}

.column-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
  padding: 0 4px;
}
.column-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 700;
  color: #4a5568;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.column-count {
  background: #cbd5e0;
  color: #ffffff;
  font-size: 12px;
  font-weight: 700;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.card {
  background: #ffffff;
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 12px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
  border: 1px solid #e2e8f0;
}
.card:last-child { margin-bottom: 0; }

.card-labels {
  display: flex;
  gap: 6px;
  margin-bottom: 10px;
}
.label {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 600;
}
.label-blue { background: #bee3f8; color: #2b6cb0; }
.label-green { background: #c6f6d5; color: #276749; }
.label-red { background: #fed7d7; color: #9b2c2c; }
.label-yellow { background: #fefcbf; color: #975a16; }
.label-purple { background: #e9d8fd; color: #553c9a; }

.card-title { font-size: 15px; font-weight: 600; color: #1a202c; margin-bottom: 6px; }
.card-desc { font-size: 13px; color: #718096; line-height: 1.5; margin-bottom: 12px; }

.card-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.card-assignee {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  font-weight: 600;
  color: #ffffff;
}
.card-meta {
  font-size: 12px;
  color: #a0aec0;
}
    "##;

    render_app("10-kanban-board", css, || view! {
        <div class="board-header">
            <h1>
                "Project Board"
            </h1>
            <div class="info">
                "Sprint 14 · Dec 9 – Dec 23"
            </div>
        </div>
        <div class="board">
            <div class="column">
                <div class="column-header">
                    <div class="column-title">
                        <span>
                            "📋 Todo"
                        </span>
                        <span class="column-count">
                            "4"
                        </span>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-blue">
                            "Feature"
                        </span>
                        <span class="label label-yellow">
                            "High Priority"
                        </span>
                    </div>
                    <div class="card-title">
                        "User authentication flow"
                    </div>
                    <div class="card-desc">
                        "Implement OAuth 2.0 login with Google and GitHub providers."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#4299e1">
                            "AL"
                        </div>
                        <div class="card-meta">
                            "2 subtasks"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-red">
                            "Bug"
                        </span>
                    </div>
                    <div class="card-title">
                        "Fix pagination offset"
                    </div>
                    <div class="card-desc">
                        "Page 2 shows duplicate results from page 1."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#f56565">
                            "BW"
                        </div>
                        <div class="card-meta">
                            "0 subtasks"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-blue">
                            "Feature"
                        </span>
                    </div>
                    <div class="card-title">
                        "Dashboard analytics"
                    </div>
                    <div class="card-desc">
                        "Add charts and metrics to the admin dashboard view."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#48bb78">
                            "CJ"
                        </div>
                        <div class="card-meta">
                            "3 subtasks"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-purple">
                            "Design"
                        </span>
                    </div>
                    <div class="card-title">
                        "Update brand guidelines"
                    </div>
                    <div class="card-desc">
                        "Refresh color palette and typography for v2 launch."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#9f7aea">
                            "DM"
                        </div>
                        <div class="card-meta">
                            "1 subtask"
                        </div>
                    </div>
                </div>
            </div>
            <div class="column">
                <div class="column-header">
                    <div class="column-title">
                        <span>
                            "🔄 In Progress"
                        </span>
                        <span class="column-count">
                            "3"
                        </span>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-blue">
                            "Feature"
                        </span>
                        <span class="label label-green">
                            "Backend"
                        </span>
                    </div>
                    <div class="card-title">
                        "API rate limiting"
                    </div>
                    <div class="card-desc">
                        "Implement token bucket rate limiting for public endpoints."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#ed8936">
                            "EB"
                        </div>
                        <div class="card-meta">
                            "1 subtask"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-yellow">
                            "High Priority"
                        </span>
                        <span class="label label-red">
                            "Bug"
                        </span>
                    </div>
                    <div class="card-title">
                        "Memory leak in worker"
                    </div>
                    <div class="card-desc">
                        "Background job worker memory grows unbounded after 24hrs."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#4299e1">
                            "AL"
                        </div>
                        <div class="card-meta">
                            "0 subtasks"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-green">
                            "Backend"
                        </span>
                    </div>
                    <div class="card-title">
                        "Database migration tool"
                    </div>
                    <div class="card-desc">
                        "Build automated migration runner for schema updates."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#38b2ac">
                            "FK"
                        </div>
                        <div class="card-meta">
                            "2 subtasks"
                        </div>
                    </div>
                </div>
            </div>
            <div class="column">
                <div class="column-header">
                    <div class="column-title">
                        <span>
                            "✅ Done"
                        </span>
                        <span class="column-count">
                            "3"
                        </span>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-blue">
                            "Feature"
                        </span>
                    </div>
                    <div class="card-title">
                        "Email notification service"
                    </div>
                    <div class="card-desc">
                        "Set up transactional email with templates using SendGrid."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#48bb78">
                            "CJ"
                        </div>
                        <div class="card-meta">
                            "Completed"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-red">
                            "Bug"
                        </span>
                    </div>
                    <div class="card-title">
                        "Fix CORS headers"
                    </div>
                    <div class="card-desc">
                        "API was missing Access-Control headers for preflight requests."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#f56565">
                            "BW"
                        </div>
                        <div class="card-meta">
                            "Completed"
                        </div>
                    </div>
                </div>
                <div class="card">
                    <div class="card-labels">
                        <span class="label label-purple">
                            "Design"
                        </span>
                        <span class="label label-blue">
                            "Feature"
                        </span>
                    </div>
                    <div class="card-title">
                        "Component library docs"
                    </div>
                    <div class="card-desc">
                        "Document all UI components with examples and usage guidelines."
                    </div>
                    <div class="card-footer">
                        <div class="card-assignee" style:background="#9f7aea">
                            "DM"
                        </div>
                        <div class="card-meta">
                            "Completed"
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }, out);
}

fn main() {
    let root = find_project_root();
    let out_dir = root.join("tests/pixel_apps/screenshots/framework");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.to_str().unwrap();

    println!("Rendering 10 apps using view! macro framework...");
    render_01_landing_page(out);
    render_02_pricing_table(out);
    render_03_login_form(out);
    render_04_profile_card(out);
    render_05_navigation_bar(out);
    render_06_data_table(out);
    render_07_dashboard_stats(out);
    render_08_blog_post(out);
    render_09_settings_panel(out);
    render_10_kanban_board(out);
    println!("Done! Screenshots saved to tests/pixel_apps/screenshots/framework/");
}

