<p align="center">
  <img src="assets/logo/logo-horizontal.svg" width="340" alt="لوگوی چیدا">
</p>

<h1 align="center">چیدا</h1>

<p align="center">
  <strong>صفحه را طراحی کن، چیدا اپ را می‌سازد.</strong><br>
  یک سازندهٔ بصری فول‌استک که با Rust نوشته شده است. صفحه‌ها، جدول‌های داده و
  جریان‌های منطق را در یک پروژه طراحی می‌کنی و در خروجی، یا تصویر رندرشدهٔ
  طرح را می‌گیری یا یک اپلیکیشن کامل و واقعی Rust که مال خودت است: بک‌اند
  Axum، فرانت‌اند Leptos، اسکیمای SQL و سند OpenAPI.
</p>

<p align="center">
  <a href="https://github.com/E30TZ/chida/actions/workflows/ci.yml"><img src="https://github.com/E30TZ/chida/actions/workflows/ci.yml/badge.svg" alt="وضعیت CI"></a>
  <a href="https://github.com/E30TZ/chida/releases/latest"><img src="https://img.shields.io/github/v/release/E30TZ/chida" alt="آخرین نسخه"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green.svg" alt="مجوز MIT"></a>
</p>

<p align="center">
  <a href="README.md">English</a> · <strong>فارسی</strong>
</p>

<p align="center">
  <img src="docs/shots/editor.png" width="880" alt="ویرایشگر وب چیدا با پروژهٔ فروشگاه فارسی">
</p>

<p align="center"><em>ویرایشگر وب چیدا: یک ابزار طراحی واقعی که در مرورگر اجرا می‌شود؛ این‌جا با قالب فروشگاه فارسی و راست‌چین.</em></p>

فیگما به تو یک عکس می‌دهد و سایت‌سازها یک صفحهٔ میزبانی‌شده. چیدا خود اپ را
می‌دهد: طرح، اسکیمای دیتابیس، API و سورس‌کد کامل، در قالب یک ورک‌اسپیس Rust
که خودت می‌سازی، اجرا می‌کنی و منتشر می‌کنی. فارسی و عربی در چیدا شهروند
درجه‌یک‌اند: متن با HarfBuzz شکل می‌گیرد و در خروجی PNG و SVG و وب،
راست‌به‌چپ رندر می‌شود و فونت وزیرمتن هم همراهش است.

## دانلود

فایل‌های آماده روی [صفحهٔ ریلیزها](https://github.com/E30TZ/chida/releases)
هستند. جدول زیر اسم دقیق فایل‌های نسخهٔ فعلی، یعنی 1.0.1، را نشان می‌دهد.
همهٔ فایل‌ها را CI از روی تگ نسخه می‌سازد و فایل `SHA256SUMS` هم برای
بررسی سلامت دانلود کنارشان است.

| سیستم | فایل | توضیح |
| --- | --- | --- |
| لینوکس، همهٔ توزیع‌ها (x86_64) | `chida-1.0.1-linux-x86_64.tar.gz` | تاربال قابل حمل: باینری چیدا، فایل دسکتاپ و `install.sh` برای نصب در `~/.local/bin` |
| لینوکس، همهٔ توزیع‌ها (x86_64) | `chida-1.0.1-linux-x86_64.AppImage` | نسخهٔ تک‌فایلی؛ بدون نصب، فقط اجراپذیرش کن و اجرا کن |
| دبیان، اوبونتو و مشتقات | `chida_1.0.1_amd64.deb` | بستهٔ دبیان: نصب در `/usr/bin/chida` با فایل دسکتاپ و آیکون |
| فدورا، RHEL و مشتقات | `chida-1.0.1-1.fc45.x86_64.rpm` | بستهٔ RPM که در کانتینر فدورا 45 ساخته می‌شود |
| لینوکس، همهٔ توزیع‌ها (aarch64) | `chida-1.0.1-linux-aarch64.tar.gz` | تاربال قابل حمل برای دستگاه‌های ARM64 |
| ویندوز (x86_64) | `chida-1.0.1-windows-x86_64.zip` | فایل زیپ شامل `chida.exe` |
| مک، Apple Silicon | `chida-1.0.1-macos-aarch64.tar.gz` | تاربال شامل باینری چیدا |
| مک، اینتل | `chida-1.0.1-macos-x86_64.tar.gz` | تاربال شامل باینری چیدا |
| هر سیستم با مرورگر | `chida-editor-1.0.1.zip` | ویرایشگر وب به‌صورت فایل‌های استاتیک (HTML و JS و WebAssembly) |

اپ دسکتاپ Tauri در پوشهٔ `apps/desktop` هنوز جزو دانلودها نیست؛ در این
نسخه فقط ابزار خط فرمان و ویرایشگر مرورگری منتشر شده‌اند.

## شروع سریع

فقط به تول‌چین Rust نیاز داری (rustup کافی است). برای CLI و کتابخانه‌ها
وابستگی سیستمی دیگری لازم نیست.

```sh
cargo build --release
./target/release/chida new demo --out examples-output
./target/release/chida info examples-output/demo.chida.json
./target/release/chida render examples-output/demo.chida.json --out examples-output/render
./target/release/chida generate examples-output/demo.chida.json --out examples-output/generated-app
cargo test
```

بعد `examples-output/render/home.png` طرح را نشان می‌دهد،
`examples-output/render/home.svg` نسخهٔ برداری است و
`examples-output/generated-app/preview/index.html` پیش‌نمایش استاتیک اپ
ساخته‌شده. بک‌اند ساخته‌شده هم داخل پوشهٔ `generated-app` با
`cargo run -p backend` اجرا می‌شود (بار اول برای گرفتن Axum و Leptos از
crates.io به اینترنت نیاز دارد).

```text
chida new <name> [--template shop|blank|persian|digikala|liquid-glass|saas-dashboard|clinic|restaurant|portfolio] [--out <dir>]
chida templates
chida render <project.chida.json> [--out <dir>]
chida generate <project.chida.json> [--out <dir>]
chida info <project.chida.json>
```

## چه چیزهایی الان کار می‌کند

* **مدل سند** (`chida-core`): صفحه‌ها، درخت گره‌ها، کامپوننت‌ها، تم‌های
  طراحی، فایل پروژهٔ JSON با پسوند `.chida.json`، اعتبارسنجی، تاریخچهٔ
  undo و redo و چیدمان خودکار (auto layout) که بین ویرایشگر و رندرر مشترک
  است.
* **رندر** (`chida-render`): رندرر نرم‌افزاری که هر صفحه را به PNG تبدیل
  می‌کند (شکل‌ها، گرادیان، سایه، بردر، شفافیت و متن واقعی TrueType) و
  خروجی SVG با متن واقعی هم می‌دهد. متن بلند داخل گره‌اش wrap می‌شود و
  خط‌های دوزبانه به ران‌های bidi تقسیم می‌شوند تا عدد و کلمهٔ لاتین وسط
  متن فارسی سر جایش بماند.
* **مدل داده** (`chida-db`): جدول با ستون‌های تایپ‌دار، قانون
  اعتبارسنجی، رابطه، دادهٔ اولیه، استور درون‌حافظه‌ای و تولید SQL
  (DDL اسکولایت، insert و select) و دستورهای migration ساده.
* **جریان منطق** (`chida-logic`): فلو به‌صورت داده: رویداد، درخت شرط
  (مقایسه، و، یا، نقیض) و اکشن‌ها: رفتن به صفحه، نمایش پیام، ست کردن و
  زیاد کردن مقدار، ذخیره و حذف رکورد، صدا زدن API و لاگ.
* **تولید کد** (`chida-codegen`): از یک پروژه، یک ورک‌اسپیس کامل Rust:
  بک‌اند Axum با روت‌های CRUD برای هر جدول به‌همراه ثبت‌نام و ورود،
  فرانت‌اند Leptos با یک کامپوننت برای هر صفحه، اسکیمای SQL، سند
  OpenAPI 3، پیش‌نمایش HTML استاتیک بدون وابستگی، Dockerfile، یونیت
  systemd و README.
* **ویرایشگر وب** (`apps/editor`): ابزار طراحی واقعی در مرورگر، نه ماکاپ.
  رابط تیره به سبک فیگما، نوار ابزار، پنل صفحه‌ها و لایه‌ها، خط‌کش،
  راهنمای snap، انتخاب چندتایی و انتخاب با کشیدن کادر، کنترل‌های چیدمان
  خودکار، استایل‌های نامی، آپلود عکس، حالت پیش‌نمایش تمام‌صفحه با ناوبری
  بین صفحه‌ها، تب‌های طراحی و داده و منطق و کد، سه سطح ساده و سازنده و
  حرفه‌ای تا تازه‌کارها غرق نشوند و ذخیرهٔ خودکار در localStorage.
* **اپ‌های ساخته‌شده کامپایل می‌شوند**: بک‌اندهای Axum ساخته‌شده با
  `cargo build` بالا می‌آیند و زنده تست شده‌اند (`/health` جواب ok داده و
  `/api/products` محصولات اولیه را برگردانده است). فرانت‌اند Leptos
  ساخته‌شده روی تارگت هاست `cargo check` را رد می‌کند؛ بیلد wasm32 با
  Trunk برای فرانت‌اند ساخته‌شده در این محیط بررسی نشده است.

## قالب‌ها

نه قالب آماده؛ هر کدام یک پروژهٔ واقعی با صفحه‌ها، دیتابیس سیدشده و دست‌کم
یک فلو منطق. فهرست را با `chida templates` ببین و جزئیات در
`templates/README.md` است.

| قالب | چه می‌سازد |
| --- | --- |
| `shop` | فروشگاه و پنل ادمین، اسکیمای محصولات و سفارش‌ها، فلو پرداخت |
| `persian` | صفحهٔ نمونهٔ فارسی راست‌چین با عدد فارسی و دکمهٔ اقدام |
| `digikala` | «بازارینو»: فروشگاه فارسی راست‌چین با صفحه‌های خانه، دسته‌بندی، محصول و سبد خرید و محصولات فارسی. برند نمونه، ساختگی و اورجینال است |
| `liquid-glass` | خانوادهٔ چهارصفحه‌ای شیشهٔ مایع: لندینگ، داشبورد، فروشگاه و موبایل |
| `saas-dashboard` | داشبورد ادمین تیره با KPI، نمودار فروش و جدول‌های سفارش و مشتری |
| `clinic` | پزشک‌ها، خدمت‌ها و فرم و فلو رزرو نوبت |
| `restaurant` | غذاهای ویژه، فرم رزرو و صفحهٔ منوی کامل |
| `portfolio` | گرید نمونه‌کارها، خدمت‌ها و فرم تماس |
| `blank` | یک صفحهٔ خالی با یک عنوان |

## گالری

رندرهای واقعی که رندرر چیدا از پروژه‌های قالب‌ها گرفته است. همهٔ عکس‌های
زیر خروجی PNG دستور `chida render` هستند، نه ماکاپ.

**خانهٔ بازارینو، فروشگاه فارسی راست‌چین** (`chida new shop2 --template digikala`)

![خانهٔ بازارینو](docs/shots/digikala-home.png)

**صفحهٔ فارسی، شکل‌گرفته و راست‌به‌چپ** (`chida new fa --template persian`)

![رندر فارسی](docs/shots/persian.png)

**ویترین فروشگاه نمونه** (`chida new demo`)

![ویترین فروشگاه نمونهٔ چیدا](docs/shots/demo-shop.png)

**لندینگ شیشهٔ مایع** (`chida new glassy --template liquid-glass`)

![خانهٔ شیشهٔ مایع](docs/shots/liquid-glass-home.png)

**موبایل شیشهٔ مایع** (صفحهٔ `/mobile` همان پروژه)

![موبایل شیشهٔ مایع](docs/shots/liquid-glass-mobile.png)

**داشبورد SaaS، پنل ادمین تیره** (`chida new panel --template saas-dashboard`)

![داشبورد SaaS](docs/shots/saas-dashboard.png)

## اپ دسکتاپ

یک پوستهٔ دسکتاپ Tauri 2 با رابط Leptos در `apps/desktop` هست. کدش کامل
است، اما در این محیط کامپایل و بسته‌بندی نشده (کتابخانه‌های توسعهٔ
WebKitGTK و GTK نصب نیستند)، پس هنوز اینستالر دسکتاپ نداریم. مراحل ساخت
برای فدورا 45 در `apps/desktop/README.md` آمده است.

## محدودیت‌های صادقانه

* اپ‌های ساخته‌شده داده را در حافظه نگه می‌دارند و موقع شروع سید
  می‌شوند. API و اسکیما کامل‌اند؛ ذخیره‌سازی ماندگار یک تعویض مستند در
  فایل `state.rs` کد ساخته‌شده است.
* هش کردن پسورد در کد ساخته‌شده SHA-256 نمک‌دار است و در خود سورس هم
  برچسب stub خورده است. برای محصول نهایی از Argon2id استفاده کن.
* پاراگراف‌های خیلی طولانیِ مخلوط فارسی و لاتین داخل یک گره ممکن است به
  تمیزی خط‌شکن یک مرورگر کامل درنیایند. لیبل، تیتر، قیمت و خط‌های معمول
  متن در هر دو خط درست رندر می‌شوند.
* پوستهٔ دسکتاپ هنوز کامپایل و بسته‌بندی نشده است (بالا را ببین).

## نقشهٔ راه

* بسته‌بندی اپ دسکتاپ Tauri (اینستالر لینوکس، ویندوز و مک).
* ذخیره‌سازی ماندگار (rusqlite یا sqlx) در اپ‌های ساخته‌شده و احراز
  هویت Argon2id.
* چیدمان پاراگراف کامل در رندرر.
* سیستم پلاگین WASM و پک‌های قالب قابل نصب.
* همکاری هم‌زمان با همگام‌سازی CRDT.
* قالب‌های بیشتر: مدرسه، CRM، وبلاگ، املاک، فیتنس و شرکتی.

## نقشهٔ ریپو

```text
crates/chida-core     مدل سند و صحنه، فایل پروژه، تاریخچه
crates/chida-render   رندر PNG و SVG
crates/chida-db       جدول‌ها، اعتبارسنجی، استور، تولید SQL
crates/chida-logic    موتور فلو بصری
crates/chida-codegen  تولید اپ فول‌استک Rust
crates/chida-cli      ابزار خط فرمان چیدا
apps/editor           ویرایشگر وب (Leptos و WebAssembly)
apps/desktop          پوستهٔ Tauri 2 (هنوز بسته‌بندی نشده)
templates             پروژه‌های قالب آماده و فایل‌هایشان
docs                  اسکرین‌شات‌ها و یادداشت‌ها
packaging             اسپک فدورا و فایل دسکتاپ
assets                لوگو و فایل‌های برند
```

## مشارکت

[CONTRIBUTING.md](CONTRIBUTING.md) را ببین. گزارش باگ و ایدهٔ قالب تازه
خوشحالمان می‌کند.

## مجوز

MIT. ببین [LICENSE](LICENSE).

فونت وزیرمتن همراه پروژه تحت مجوز SIL Open Font License 1.1 است و DejaVu
Sans فونت جایگزین است.
