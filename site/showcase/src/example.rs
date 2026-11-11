use openui::prelude::*;
use openui_components::*;

pub fn deploy_card() -> ViewNode {
    let (primary, outline) = (ButtonVariant::Default, ButtonVariant::Outline);
    view! {
        <div
            style:box-sizing="border-box"
            style:width="360px"
            style:padding="24px"
            style:border="1px solid #e5e5e5"
            style:border-radius="14px"
        >
            <div style:display="flex" style:justify-content="space-between">
                <div style:font-size="16px" style:font-weight="600">
                    "Production"
                </div>
                <Badge variant={BadgeVariant::Secondary}>"Live"</Badge>
            </div>
            <div style:margin="4px 0 20px" style:color="#737373">
                "Deployed 4 minutes ago from main"
            </div>
            <div style:display="flex" style:gap="8px">
                <Button variant={primary} size={ButtonSize::Sm} disabled={false}>
                    "Redeploy"
                </Button>
                <Button variant={outline} size={ButtonSize::Sm} disabled={false}>
                    "View logs"
                </Button>
            </div>
        </div>
    }
}
