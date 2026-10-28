use crate::models::EcommerceStackProfile;

pub fn detect_ecommerce_stack(html: &str) -> EcommerceStackProfile {
    let lower = html.to_lowercase();
    let mut payment_gateways = Vec::new();
    let mut live_chats = Vec::new();
    let mut store_engines = Vec::new();

    // 1. Payment Gateways
    if lower.contains("yookassa")
        || lower.contains("kassa.yandex.ru")
        || lower.contains("yoomoney.ru")
    {
        payment_gateways.push("ЮKassa (YooKassa)".to_string());
    }
    if lower.contains("securepay.tinkoff.ru")
        || lower.contains("tinkoff.ru/kassa")
        || lower.contains("t-bank.ru/kassa")
    {
        payment_gateways.push("Т-Банк Оплата (Tinkoff Pay)".to_string());
    }
    if lower.contains("widget.cloudpayments.ru") || lower.contains("cloudpayments") {
        payment_gateways.push("CloudPayments".to_string());
    }
    if lower.contains("auth.robokassa.ru") || lower.contains("robokassa") {
        payment_gateways.push("Robokassa".to_string());
    }
    if lower.contains("js.stripe.com") || lower.contains("api.stripe.com") {
        payment_gateways.push("Stripe".to_string());
    }
    if lower.contains("paypal.com/sdk") || lower.contains("paypalobjects.com") {
        payment_gateways.push("PayPal".to_string());
    }
    if lower.contains("payonline.ru") {
        payment_gateways.push("PayOnline".to_string());
    }

    // 2. Live Chat & Customer Communication Widgets
    if lower.contains("code.jivosite.com") || lower.contains("jivo.ru") {
        live_chats.push("JivoChat (JivoSite)".to_string());
    }
    if lower.contains("carrotquest.io") || lower.contains("cdn.carrotquest.app") {
        live_chats.push("Carrot Quest".to_string());
    }
    if lower.contains("talk-me.ru") {
        live_chats.push("Talk-Me".to_string());
    }
    if lower.contains("widget.intercom.io") {
        live_chats.push("Intercom".to_string());
    }
    if lower.contains("cdn.livechatinc.com") {
        live_chats.push("LiveChat".to_string());
    }
    if lower.contains("marquiz.ru") {
        live_chats.push("Marquiz (Квизы)".to_string());
    }
    if lower.contains("envybox.io") {
        live_chats.push("Envybox".to_string());
    }

    // 3. Store Engines & E-commerce Footprints
    if lower.contains("t-store") || lower.contains("tilda-store") {
        store_engines.push("Tilda Store".to_string());
    }
    if lower.contains("insales.ru") || lower.contains("static.insales.ru") {
        store_engines.push("InSales".to_string());
    }
    if lower.contains("woocommerce") || lower.contains("wc-api") {
        store_engines.push("WooCommerce".to_string());
    }
    if lower.contains("bitrix.eshop") || lower.contains("catalog.element") {
        store_engines.push("1C-Bitrix E-Shop".to_string());
    }
    if lower.contains("app.ecwid.com") {
        store_engines.push("Ecwid".to_string());
    }
    if lower.contains("cdn.shopify.com") || lower.contains("myshopify.com") {
        store_engines.push("Shopify".to_string());
    }

    let is_commercial = !payment_gateways.is_empty() || !store_engines.is_empty();

    EcommerceStackProfile {
        payment_gateways,
        live_chats,
        store_engines,
        is_commercial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecommerce_payment_detection() {
        let html = r#"
            <html>
                <head>
                    <script src="https://static.yoomoney.ru/checkout-js/v1/checkout.js"></script>
                    <script src="//code.jivosite.com/widget/abc123"></script>
                </head>
                <body>
                    <div class="t-store__prod-popup">Купить товар</div>
                </body>
            </html>
        "#;
        let profile = detect_ecommerce_stack(html);
        assert!(profile.is_commercial);
        assert!(
            profile
                .payment_gateways
                .iter()
                .any(|p| p.contains("YooKassa"))
        );
        assert!(profile.live_chats.iter().any(|c| c.contains("JivoChat")));
        assert!(
            profile
                .store_engines
                .iter()
                .any(|s| s.contains("Tilda Store"))
        );
    }

    #[test]
    fn test_tinkoff_and_carrot_detection() {
        let html = r#"
            <script src="https://securepay.tinkoff.ru/html/payForm/js/tinkoff_v2.js"></script>
            <script src="https://cdn.carrotquest.app/api.carrotquest.io/apps/xyz/widget.js"></script>
        "#;
        let profile = detect_ecommerce_stack(html);
        assert!(profile.is_commercial);
        assert!(
            profile
                .payment_gateways
                .iter()
                .any(|p| p.contains("Т-Банк"))
        );
        assert!(
            profile
                .live_chats
                .iter()
                .any(|c| c.contains("Carrot Quest"))
        );
    }
}
