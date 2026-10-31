fn Navbar() -> Element {
    return <nav style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        justifyContent: "space-between",
        gap: "16px 24px",
        padding: "16px clamp(16px, 4vw, 32px)",
        borderBottom: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "#09090b",
    }}>
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
            <div style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: "36px",
                height: "36px",
                borderRadius: "12px",
                backgroundColor: "#8b5cf6",
                color: "#ffffff",
                fontSize: "20px",
                fontWeight: "900",
            }}>
                R
            </div>
            <span style={{
                fontSize: "20px",
                fontWeight: "700",
                letterSpacing: "-0.5px",
                color: "#ffffff",
            }}>
                RustX<span style={{ color: "#a78bfa" }}>.</span>
            </span>
            <span style={{
                fontSize: "11px",
                color: "#71717a",
                border: "1px solid rgba(255,255,255,0.1)",
                borderRadius: "999px",
                padding: "4px 8px",
            }}>
                EARLY ACCESS
            </span>
        </div>

        <div style={{
            display: "flex",
            flexWrap: "wrap",
            alignItems: "center",
            gap: "12px 24px",
            fontSize: "14px",
            color: "#a1a1aa",
        }}>
            <a href="#features" style={{ color: "inherit", textDecoration: "none" }}>Features</a>
            <a href="#example" style={{ color: "inherit", textDecoration: "none" }}>Documentation</a>
            <a href="#ecosystem" style={{ color: "inherit", textDecoration: "none" }}>Ecosystem</a>
        </div>

        <a href="#start" style={{
            display: "inline-block",
            borderRadius: "8px",
            backgroundColor: "#ffffff",
            color: "#09090b",
            padding: "10px 16px",
            fontSize: "14px",
            fontWeight: "600",
            textDecoration: "none",
        }}>
            Get started ↗
        </a>
    </nav>;
}

fn CodePreview() -> Element {
    return <div style={{
        width: "100%",
        minWidth: "0",
        maxWidth: "560px",
        border: "1px solid rgba(255,255,255,0.1)",
        borderRadius: "16px",
        backgroundColor: "#101014",
        boxShadow: "0 25px 60px rgba(76,29,149,0.25)",
        overflow: "hidden",
    }}>
        <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "12px 16px",
            borderBottom: "1px solid rgba(255,255,255,0.1)",
        }}>
            <div style={{ display: "flex", gap: "8px" }}>
                <div style={{ width: "12px", height: "12px", borderRadius: "50%", backgroundColor: "#f87171" }}></div>
                <div style={{ width: "12px", height: "12px", borderRadius: "50%", backgroundColor: "#facc15" }}></div>
                <div style={{ width: "12px", height: "12px", borderRadius: "50%", backgroundColor: "#4ade80" }}></div>
            </div>
            <span style={{ fontSize: "12px", color: "#71717a" }}>src/main.rsx</span>
            <span style={{ fontSize: "12px", color: "#a78bfa" }}>RustX</span>
        </div>

        <pre style={{
            margin: "0",
            padding: "clamp(14px, 3vw, 24px)",
            fontSize: "clamp(11px, 1.2vw, 13px)",
            lineHeight: "1.9",
            overflowX: "auto",
            color: "#e4e4e7",
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
        }}>
            <code>
                <span style={{ color: "#a78bfa" }}>fn</span>{" "}
                <span style={{ color: "#fde68a" }}>Counter</span>
                <span style={{ color: "#a1a1aa" }}>() -&gt; Element {"{"}</span>
                {"\n    "}
                <span style={{ color: "#a78bfa" }}>let mut</span>{" "}
                <span style={{ color: "#7dd3fc" }}>count</span>{" = "}
                <span style={{ color: "#fdba74" }}>0</span>
                {";\n\n    "}
                <span style={{ color: "#a78bfa" }}>return</span>{" "}
                <span style={{ color: "#fca5a5" }}>&lt;div</span>
                <span style={{ color: "#7dd3fc" }}> className</span>
                <span style={{ color: "#a1a1aa" }}>=</span>
                <span style={{ color: "#86efac" }}>"card"</span>
                <span style={{ color: "#fca5a5" }}>&gt;</span>
                {"\n        "}
                <span style={{ color: "#fca5a5" }}>&lt;h1&gt;</span>
                <span style={{ color: "#ffffff" }}>Hello, RustX.</span>
                <span style={{ color: "#fca5a5" }}>&lt;/h1&gt;</span>
                {"\n\n        "}
                <span style={{ color: "#fca5a5" }}>&lt;button</span>
                {"\n            "}
                <span style={{ color: "#7dd3fc" }}>onClick</span>
                <span style={{ color: "#a1a1aa" }}>=</span>
                <span style={{ color: "#86efac" }}>{"{|| count += 1}"}</span>
                {"\n        "}
                <span style={{ color: "#fca5a5" }}>&gt;</span>
                {"\n            "}
                <span style={{ color: "#ffffff" }}>Clicked: </span>
                <span style={{ color: "#7dd3fc" }}>{"{count}"}</span>
                {"\n        "}
                <span style={{ color: "#fca5a5" }}>&lt;/button&gt;</span>
                {"\n    "}
                <span style={{ color: "#fca5a5" }}>&lt;/div&gt;</span>
                {";\n"}
                <span style={{ color: "#a1a1aa" }}>{"}"}</span>
            </code>
        </pre>

        <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "12px 20px",
            borderTop: "1px solid rgba(255,255,255,0.1)",
            backgroundColor: "rgba(255,255,255,0.02)",
        }}>
            <span style={{ fontSize: "12px", color: "#71717a" }}>● Native compilation</span>
            <span style={{ fontSize: "12px", color: "#34d399" }}>Ready to build</span>
        </div>
    </div>;
}

fn FeatureCard(title: &str, description: &str, icon: &str) -> Element {
    return <div style={{
        border: "1px solid rgba(255,255,255,0.1)",
        borderRadius: "16px",
        backgroundColor: "rgba(255,255,255,0.02)",
        padding: "24px",
    }}>
        <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            width: "44px",
            height: "44px",
            borderRadius: "12px",
            backgroundColor: "rgba(139,92,246,0.1)",
            color: "#c4b5fd",
            fontSize: "22px",
            marginBottom: "20px",
        }}>
            {icon}
        </div>
        <h3 style={{
            margin: "0 0 8px",
            fontSize: "18px",
            fontWeight: "600",
            color: "#ffffff",
        }}>{title}</h3>
        <p style={{
            margin: "0",
            fontSize: "14px",
            lineHeight: "1.7",
            color: "#a1a1aa",
        }}>{description}</p>
    </div>;
}

fn Page() -> Element {
    return <div style={{
        minHeight: "100vh",
        width: "100%",
        overflowX: "hidden",
        backgroundColor: "#09090b",
        color: "#ffffff",
        fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif",
    }}>
        {Navbar()}
        <main>
            <section style={{
                position: "relative",
                overflow: "hidden",
                padding: "clamp(64px, 9vw, 112px) clamp(16px, 4vw, 24px)",
            }}>
                <div style={{
                    position: "absolute",
                    width: "384px",
                    height: "384px",
                    borderRadius: "50%",
                    backgroundColor: "rgba(124,58,237,0.18)",
                    filter: "blur(120px)",
                    top: "-96px",
                    left: "50%",
                    transform: "translateX(-50%)",
                    pointerEvents: "none",
                }}></div>

                <div style={{
                    position: "relative",
                    maxWidth: "1280px",
                    margin: "0 auto",
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 420px), 1fr))",
                    gap: "clamp(32px, 5vw, 64px)",
                    alignItems: "center",
                }}>
                    <div>
                        <div style={{
                            display: "inline-flex",
                            alignItems: "center",
                            gap: "8px",
                            border: "1px solid rgba(167,139,250,0.2)",
                            backgroundColor: "rgba(167,139,250,0.05)",
                            borderRadius: "999px",
                            padding: "6px 12px",
                            fontSize: "12px",
                            color: "#c4b5fd",
                            marginBottom: "32px",
                        }}>
                            <span style={{
                                width: "8px",
                                height: "8px",
                                borderRadius: "50%",
                                backgroundColor: "#a78bfa",
                            }}></span>
                            The native app framework for Rust
                        </div>

                        <h1 style={{
                            margin: "0 0 28px",
                            fontSize: "clamp(40px, 7vw, 76px)",
                            fontWeight: "750",
                            letterSpacing: "clamp(-3px, -0.25vw, -1px)",
                            lineHeight: "1.05",
                        }}>
                            Write less.
                            <br />
                            <span style={{
                                backgroundImage: "linear-gradient(90deg, #a78bfa, #d8b4fe, #818cf8)",
                                backgroundClip: "text",
                                WebkitBackgroundClip: "text",
                                color: "transparent",
                            }}>
                                Build native.
                            </span>
                        </h1>

                        <p style={{
                            maxWidth: "500px",
                            margin: "0 0 36px",
                            fontSize: "clamp(16px, 2vw, 18px)",
                            lineHeight: "1.8",
                            color: "#a1a1aa",
                        }}>
                            Your UI. Your logic. One language.
                            Build desktop applications with the syntax you love
                            and the power of Rust.
                        </p>

                        <div style={{ display: "flex", flexWrap: "wrap", gap: "12px" }}>
                            <a href="#start" style={{
                                display: "inline-block",
                                padding: "13px 24px",
                                borderRadius: "12px",
                                backgroundColor: "#8b5cf6",
                                color: "#ffffff",
                                fontWeight: "600",
                                textDecoration: "none",
                            }}>
                                Start building →
                            </a>
                            <a href="#example" style={{
                                display: "inline-block",
                                padding: "13px 24px",
                                border: "1px solid rgba(255,255,255,0.1)",
                                borderRadius: "12px",
                                color: "#f4f4f5",
                                fontWeight: "500",
                                textDecoration: "none",
                            }}>
                                Explore the code
                            </a>
                        </div>

                        <div style={{
                            display: "flex",
                            flexWrap: "wrap",
                            gap: "24px",
                            marginTop: "40px",
                            fontSize: "13px",
                            color: "#71717a",
                        }}>
                            <span>⚡ Rust-powered</span>
                            <span>⌘ JSX-inspired syntax</span>
                            <span>◈ npm ecosystem</span>
                        </div>
                    </div>

                    <div id="example" style={{ display: "flex", justifyContent: "center", minWidth: "0", width: "100%" }}>
                        {CodePreview()}
                    </div>
                </div>
            </section>

            <section style={{
                borderTop: "1px solid rgba(255,255,255,0.1)",
                borderBottom: "1px solid rgba(255,255,255,0.1)",
                backgroundColor: "rgba(255,255,255,0.015)",
                padding: "24px clamp(16px, 4vw, 24px)",
            }}>
                <div style={{
                    maxWidth: "1000px",
                    margin: "0 auto",
                    display: "flex",
                    flexWrap: "wrap",
                    justifyContent: "center",
                    alignItems: "center",
                    gap: "20px 48px",
                    fontSize: "14px",
                    color: "#71717a",
                }}>
                    <span>One component model</span>
                    <span style={{ color: "#8b5cf6" }}>✳</span>
                    <span>Native compilation</span>
                    <span style={{ color: "#8b5cf6" }}>✳</span>
                    <span>Familiar DOM-like API</span>
                    <span style={{ color: "#8b5cf6" }}>✳</span>
                    <span>Modern npm libraries</span>
                </div>
            </section>

            <section id="features" style={{
                maxWidth: "1280px",
                margin: "0 auto",
                padding: "clamp(56px, 8vw, 96px) clamp(16px, 4vw, 24px)",
            }}>
                <div style={{ maxWidth: "640px", marginBottom: "56px" }}>
                    <p style={{
                        margin: "0 0 16px",
                        fontSize: "12px",
                        fontWeight: "700",
                        textTransform: "uppercase",
                        letterSpacing: "3px",
                        color: "#a78bfa",
                    }}>
                        Designed for developers
                    </p>
                    <h2 style={{
                        margin: "0 0 20px",
                        fontSize: "clamp(32px, 4vw, 52px)",
                        lineHeight: "1.15",
                        letterSpacing: "-1.5px",
                        fontWeight: "700",
                    }}>
                        The web ecosystem.
                        <br />
                        A native-first mindset.
                    </h2>
                    <p style={{ margin: "0", color: "#a1a1aa", lineHeight: "1.8" }}>
                        Keep the tools and mental models you already know.
                        Build a different kind of desktop application.
                    </p>
                </div>

                <div style={{
                    display: "grid",
                    gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
                    gap: "16px",
                }}>
                    {FeatureCard(
                        "Rust at the core",
                        "Use Rust types, ownership and native compilation as the foundation of your application.",
                        "⚙"
                    )}
                    {FeatureCard(
                        "RSX components",
                        "Describe your interface declaratively with reusable components, props and event handlers.",
                        "⌘"
                    )}
                    {FeatureCard(
                        "npm ecosystem",
                        "Bring familiar libraries and tools into your workflow through an integration layer.",
                        "⬡"
                    )}
                    {FeatureCard(
                        "Reactive interfaces",
                        "Connect UI updates to application state with a concise, component-oriented model.",
                        "↯"
                    )}
                    {FeatureCard(
                        "One component model",
                        "Compose small, reusable elements into interfaces that remain easy to maintain.",
                        "◈"
                    )}
                    {FeatureCard(
                        "Built for desktop",
                        "Design applications around native packaging, startup time and system integration.",
                        "▣"
                    )}
                </div>
            </section>

            <section id="ecosystem" style={{
                padding: "clamp(56px, 7vw, 80px) clamp(16px, 4vw, 24px)",
                borderTop: "1px solid rgba(255,255,255,0.1)",
                borderBottom: "1px solid rgba(255,255,255,0.1)",
                backgroundColor: "#0d0c12",
            }}>
                <div style={{ maxWidth: "1000px", margin: "0 auto", textAlign: "center" }}>
                    <p style={{
                        margin: "0 0 20px",
                        fontSize: "12px",
                        textTransform: "uppercase",
                        letterSpacing: "3px",
                        color: "#a78bfa",
                    }}>
                        Your existing ecosystem
                    </p>
                    <h2 style={{
                        margin: "0 0 20px",
                        fontSize: "clamp(32px, 4vw, 52px)",
                        fontWeight: "700",
                        letterSpacing: "-1.5px",
                    }}>
                        Keep your favorite tools.
                    </h2>
                    <p style={{
                        maxWidth: "680px",
                        margin: "0 auto 48px",
                        color: "#a1a1aa",
                        lineHeight: "1.8",
                    }}>
                        Rich-text editors, workflow canvases, authentication and
                        reactive backends. RustX aims to bring the libraries you
                        already rely on into a native application workflow.
                    </p>

                    <div style={{
                        display: "flex",
                        flexWrap: "wrap",
                        justifyContent: "center",
                        gap: "12px",
                    }}>
                        <span style={{border: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        padding: "12px 20px",
        fontWeight: "600",
        color: "#e4e4e7",}}>Tiptap</span>
                        <span style={{border: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        padding: "12px 20px",
        fontWeight: "600",
        color: "#e4e4e7",}}>Xiflow</span>
                        <span style={{border: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        padding: "12px 20px",
        fontWeight: "600",
        color: "#e4e4e7",}}>Convex</span>
                        <span style={{border: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        padding: "12px 20px",
        fontWeight: "600",
        color: "#e4e4e7",}}>Supabase</span>
                        <span style={{border: "1px solid rgba(255,255,255,0.1)",
        backgroundColor: "rgba(255,255,255,0.03)",
        borderRadius: "12px",
        padding: "12px 20px",
        fontWeight: "600",
        color: "#e4e4e7",}}>npm</span>
                    </div>
                    <p style={{ margin: "24px 0 0", fontSize: "12px", color: "#52525b" }}>
                        Integration availability depends on each library's runtime requirements.
                    </p>
                </div>
            </section>

            <section id="start" style={{ padding: "96px 24px" }}>
                <div style={{
                    position: "relative",
                    maxWidth: "1000px",
                    margin: "0 auto",
                    overflow: "hidden",
                    border: "1px solid rgba(167,139,250,0.2)",
                    borderRadius: "24px",
                    background: "linear-gradient(135deg, rgba(139,92,246,0.12), #111116 55%, rgba(99,102,241,0.1))",
                    padding: "clamp(36px, 6vw, 64px) clamp(16px, 4vw, 32px)",
                    textAlign: "center",
                }}>
                    <div style={{
                        position: "absolute",
                        width: "256px",
                        height: "256px",
                        borderRadius: "50%",
                        backgroundColor: "rgba(139,92,246,0.1)",
                        filter: "blur(100px)",
                        top: "0",
                        left: "50%",
                        transform: "translateX(-50%)",
                        pointerEvents: "none",
                    }}></div>
                    <div style={{ position: "relative" }}>
                        <h2 style={{
                            margin: "0 0 20px",
                            fontSize: "clamp(32px, 4vw, 52px)",
                            fontWeight: "700",
                            letterSpacing: "-1.5px",
                        }}>
                            The next app starts here.
                        </h2>
                        <p style={{
                            maxWidth: "600px",
                            margin: "0 auto 32px",
                            color: "#a1a1aa",
                            lineHeight: "1.8",
                        }}>
                            Less boilerplate. Reusable components. Rust at the center
                            of your development workflow.
                        </p>
                        <div style={{
                            display: "inline-flex",
                            flexWrap: "wrap",
                            alignItems: "center",
                            justifyContent: "center",
                            gap: "12px",
                            maxWidth: "100%",
                            overflowWrap: "anywhere",
                            backgroundColor: "rgba(0,0,0,0.4)",
                            border: "1px solid rgba(255,255,255,0.1)",
                            borderRadius: "12px",
                            padding: "16px 20px",
                            fontFamily: "ui-monospace, SFMono-Regular, Menlo, monospace",
                            fontSize: "14px",
                        }}>
                            <span style={{ color: "#a78bfa" }}>$</span>
                            <span style={{ color: "#e4e4e7" }}>cargo install rustx-cli</span>
                        </div>
                        <p style={{ margin: "16px 0 0", fontSize: "12px", color: "#52525b" }}>
                            Example installation command. Replace with the actual RustX CLI package.
                        </p>
                    </div>
                </div>
            </section>
        </main>

        <footer style={{
            borderTop: "1px solid rgba(255,255,255,0.1)",
            padding: "28px 24px",
        }}>
            <div style={{
                maxWidth: "1280px",
                margin: "0 auto",
                display: "flex",
                flexWrap: "wrap",
                justifyContent: "space-between",
                gap: "16px",
                fontSize: "13px",
                color: "#71717a",
            }}>
                <span>© 2026 RustX. Built with Rust.</span>
                <span>Made for developers who want more control.</span>
            </div>
        </footer>
    </div>;
}