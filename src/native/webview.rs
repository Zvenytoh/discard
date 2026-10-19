//! Asynchronous controller creation and the boundary around remote content.

use super::{app, App};
use crate::navigation;
use webview2_com::{
    CoTaskMemPWSTR, CreateCoreWebView2ControllerCompletedHandler,
    CreateCoreWebView2EnvironmentCompletedHandler, DownloadStartingEventHandler,
    Microsoft::Web::WebView2::Win32::{
        CreateCoreWebView2EnvironmentWithOptions, ICoreWebView2Controller, ICoreWebView2_4,
    },
    NavigationStartingEventHandler, NewWindowRequestedEventHandler,
};
use windows::core::{Interface, HSTRING, PWSTR};

impl App {
    pub(super) fn create_webview(&self, profile: &HSTRING) -> windows::core::Result<()> {
        let handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |result, environment| {
                let Some(state) = app() else {
                    return Ok(());
                };
                if let Err(error) = result {
                    state.report_failure(error);
                    return Ok(());
                }
                let Some(environment) = environment else {
                    state.report_failure("WebView2 returned no environment.");
                    return Ok(());
                };
                let handler = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                    move |result, controller| {
                        let Some(state) = app() else {
                            if let Some(controller) = controller {
                                // SAFETY: this late controller still belongs to the STA.
                                unsafe {
                                    let _ = controller.Close();
                                }
                            }
                            return Ok(());
                        };
                        if let Err(error) = result {
                            state.report_failure(error);
                            return Ok(());
                        }
                        let Some(controller) = controller else {
                            state.report_failure("WebView2 returned no controller.");
                            return Ok(());
                        };
                        *state.controller.borrow_mut() = Some(controller.clone());
                        if let Err(error) = state.configure(&controller) {
                            state.report_failure(error);
                        }
                        Ok(())
                    },
                ));
                // SAFETY: the parent window is alive on this STA; COM retains handler.
                if let Err(error) =
                    unsafe { environment.CreateCoreWebView2Controller(state.hwnd, &handler) }
                {
                    state.report_failure(error);
                }
                Ok(())
            },
        ));
        // SAFETY: profile is valid UTF-16 for this call; COM retains the callback.
        unsafe { CreateCoreWebView2EnvironmentWithOptions(None, profile, None, &handler) }
    }

    fn configure(&self, controller: &ICoreWebView2Controller) -> windows::core::Result<()> {
        // SAFETY: controller, webview and callback arguments are all owned by this
        // STA. COM-allocated URI strings are immediately wrapped for correct freeing.
        unsafe {
            self.resize()?;
            controller.SetIsVisible(true)?;
            let web = controller.CoreWebView2()?;
            let settings = web.Settings()?;
            settings.SetIsWebMessageEnabled(false)?;
            settings.SetAreHostObjectsAllowed(false)?;
            let mut token = 0;
            web.add_NavigationStarting(
                &NavigationStartingEventHandler::create(Box::new(|_, args| {
                    if let Some(args) = args {
                        let mut raw = PWSTR::null();
                        let result = args.Uri(&mut raw);
                        let uri = CoTaskMemPWSTR::from(raw);
                        if let Err(error) = result {
                            args.SetCancel(true)?;
                            return Err(error);
                        }
                        args.SetCancel(!navigation::allowed(&uri.to_string()))?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            web.add_NewWindowRequested(
                &NewWindowRequestedEventHandler::create(Box::new(|_, args| {
                    if let Some(args) = args {
                        args.SetHandled(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            web.cast::<ICoreWebView2_4>()?.add_DownloadStarting(
                &DownloadStartingEventHandler::create(Box::new(|_, args| {
                    if let Some(args) = args {
                        args.SetCancel(true)?;
                    }
                    Ok(())
                })),
                &mut token,
            )?;
            web.Navigate(&HSTRING::from(navigation::HOME))?;
        }
        Ok(())
    }
}
