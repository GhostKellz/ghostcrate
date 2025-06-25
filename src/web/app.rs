use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/ghostcrate.css"/>
        <Title text="GhostCrate"/>

        <Router>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                    <Route path="/login" view=LoginPage/>
                    <Route path="/register" view=RegisterPage/>
                    <Route path="/dashboard" view=DashboardPage/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div class="container">
            <div class="hero">
                <h1 class="title">"👻️ GhostCrate"</h1>
                <p class="subtitle">"Self-hosted Rust crate registry & package server"</p>
                <div class="buttons">
                    <A href="/login" class="button is-primary">"Login"</A>
                    <A href="/register" class="button is-light">"Register"</A>
                </div>
            </div>
        </div>
    }
}

#[component]
fn LoginPage() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(Option::<String>::None);
    let (is_loading, set_is_loading) = create_signal(false);

    let login_action = create_action(move |_: &()| {
        let username_val = username.get();
        let password_val = password.get();
        
        async move {
            set_is_loading.set(true);
            set_error_message.set(None);
            
            let login_request = serde_json::json!({
                "username": username_val,
                "password": password_val
            });
            
            let response = gloo_net::http::Request::post("/api/auth/login")
                .json(&login_request)
                .unwrap()
                .send()
                .await;
                
            set_is_loading.set(false);
            
            match response {
                Ok(resp) if resp.ok() => {
                    // Handle successful login
                    let navigate = leptos_router::use_navigate();
                    navigate("/dashboard", Default::default());
                }
                Ok(_) => {
                    set_error_message.set(Some("Invalid username or password".to_string()));
                }
                Err(_) => {
                    set_error_message.set(Some("Network error".to_string()));
                }
            }
        }
    });

    view! {
        <div class="container">
            <div class="columns is-centered">
                <div class="column is-one-third">
                    <div class="box">
                        <h2 class="title is-4">"Login to GhostCrate"</h2>
                        
                        <Show when=move || error_message.get().is_some()>
                            <div class="notification is-danger">
                                {move || error_message.get().unwrap_or_default()}
                            </div>
                        </Show>
                        
                        <form on:submit=move |ev| {
                            ev.prevent_default();
                            login_action.dispatch(());
                        }>
                            <div class="field">
                                <label class="label">"Username"</label>
                                <div class="control">
                                    <input 
                                        class="input" 
                                        type="text" 
                                        placeholder="Enter username"
                                        prop:value=username
                                        on:input=move |ev| set_username.set(event_target_value(&ev))
                                        required
                                    />
                                </div>
                            </div>
                            
                            <div class="field">
                                <label class="label">"Password"</label>
                                <div class="control">
                                    <input 
                                        class="input" 
                                        type="password" 
                                        placeholder="Enter password"
                                        prop:value=password
                                        on:input=move |ev| set_password.set(event_target_value(&ev))
                                        required
                                    />
                                </div>
                            </div>
                            
                            <div class="field">
                                <div class="control">
                                    <button 
                                        class=move || format!("button is-primary is-fullwidth {}", if is_loading.get() { "is-loading" } else { "" })
                                        type="submit"
                                        disabled=is_loading
                                    >
                                        "Login"
                                    </button>
                                </div>
                            </div>
                        </form>
                        
                        <div class="has-text-centered">
                            <p>"Don't have an account? " <A href="/register">"Register here"</A></p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn RegisterPage() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (email, set_email) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());
    let (error_message, set_error_message) = create_signal(Option::<String>::None);
    let (is_loading, set_is_loading) = create_signal(false);

    let register_action = create_action(move |_: &()| {
        let username_val = username.get();
        let email_val = email.get();
        let password_val = password.get();
        
        async move {
            set_is_loading.set(true);
            set_error_message.set(None);
            
            let register_request = serde_json::json!({
                "username": username_val,
                "email": email_val,
                "password": password_val
            });
            
            let response = gloo_net::http::Request::post("/api/auth/register")
                .json(&register_request)
                .unwrap()
                .send()
                .await;
                
            set_is_loading.set(false);
            
            match response {
                Ok(resp) if resp.ok() => {
                    let navigate = leptos_router::use_navigate();
                    navigate("/login", Default::default());
                }
                Ok(_) => {
                    set_error_message.set(Some("Registration failed".to_string()));
                }
                Err(_) => {
                    set_error_message.set(Some("Network error".to_string()));
                }
            }
        }
    });

    view! {
        <div class="container">
            <div class="columns is-centered">
                <div class="column is-one-third">
                    <div class="box">
                        <h2 class="title is-4">"Register for GhostCrate"</h2>
                        
                        <Show when=move || error_message.get().is_some()>
                            <div class="notification is-danger">
                                {move || error_message.get().unwrap_or_default()}
                            </div>
                        </Show>
                        
                        <form on:submit=move |ev| {
                            ev.prevent_default();
                            register_action.dispatch(());
                        }>
                            <div class="field">
                                <label class="label">"Username"</label>
                                <div class="control">
                                    <input 
                                        class="input" 
                                        type="text" 
                                        placeholder="Choose a username"
                                        prop:value=username
                                        on:input=move |ev| set_username.set(event_target_value(&ev))
                                        required
                                    />
                                </div>
                            </div>
                            
                            <div class="field">
                                <label class="label">"Email"</label>
                                <div class="control">
                                    <input 
                                        class="input" 
                                        type="email" 
                                        placeholder="Enter your email"
                                        prop:value=email
                                        on:input=move |ev| set_email.set(event_target_value(&ev))
                                        required
                                    />
                                </div>
                            </div>
                            
                            <div class="field">
                                <label class="label">"Password"</label>
                                <div class="control">
                                    <input 
                                        class="input" 
                                        type="password" 
                                        placeholder="Create a password"
                                        prop:value=password
                                        on:input=move |ev| set_password.set(event_target_value(&ev))
                                        required
                                    />
                                </div>
                            </div>
                            
                            <div class="field">
                                <div class="control">
                                    <button 
                                        class=move || format!("button is-primary is-fullwidth {}", if is_loading.get() { "is-loading" } else { "" })
                                        type="submit"
                                        disabled=is_loading
                                    >
                                        "Register"
                                    </button>
                                </div>
                            </div>
                        </form>
                        
                        <div class="has-text-centered">
                            <p>"Already have an account? " <A href="/login">"Login here"</A></p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn DashboardPage() -> impl IntoView {
    view! {
        <div class="container">
            <div class="hero is-info">
                <div class="hero-body">
                    <h1 class="title">"Dashboard"</h1>
                    <p class="subtitle">"Welcome to your GhostCrate registry!"</p>
                </div>
            </div>
            
            <section class="section">
                <div class="columns">
                    <div class="column">
                        <div class="box">
                            <h3 class="title is-5">"📦 Your Crates"</h3>
                            <p>"No crates published yet. Start by publishing your first crate!"</p>
                        </div>
                    </div>
                    <div class="column">
                        <div class="box">
                            <h3 class="title is-5">"📊 Statistics"</h3>
                            <p>"Total Downloads: 0"</p>
                            <p>"Total Crates: 0"</p>
                        </div>
                    </div>
                </div>
            </section>
        </div>
    }
}