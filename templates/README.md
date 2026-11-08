# Chida templates

Templates are real Chida projects, not pictures: each one is built from
chida-core nodes, a database schema with seed rows, and logic flows, so
it renders to PNG and SVG, opens in the editor, and feeds the code
generator exactly like a project you made yourself.

List them and create one:

```sh
chida templates
chida new myshop --template digikala
chida new glassy --template liquid-glass
chida render myshop.chida.json
```

## digikala: Marketplace Pro

A Persian, fully RTL marketplace in the style of large Iranian
e-commerce sites. The sample brand is «بازارینو» (Bazaarino), invented
for this template: no Digikala logo, name, text, images or data are
used anywhere. Layout and shopping experience follow the familiar
marketplace pattern; every word, product and price is original sample
data stored in the project itself.

Pages (all 1440 wide):

- Home `/`: header with search, login and cart, category navigation,
  hero campaign banner, an amazing offers section (red accent, four
  discounted cards with old price, new price and a discount badge),
  eight category tiles, a featured product row, two promo banners, a
  best sellers row, and a full footer with link columns.
- Category `/category`: filter sidebar (brand checkboxes, price range
  slider, in-stock and fast-shipping toggles), a sort row, and a 3 by 3
  product grid with ratings.
- Product `/product`: image gallery with thumbnails, title, rating,
  colour dots, seller and price box with an add to cart button, a
  delivery box, a five row spec table, and similar products.
- Cart `/cart`: two cart items with quantity controls, a price summary
  box with discount and a checkout button, plus a suggestions row.

Database: `categories`, `products` (name, price, discount_percent,
stock, rating, category relation), `orders`, and `users`, with twelve
Persian seed products priced in Toman and rendered in Persian digits.

Flows: an add to cart flow (the product page button saves an order
record when stock is available and navigates to the cart) and a search
flow stub (submit calls the generated search API endpoint and then
navigates to the category page; wiring the endpoint to a live query is
left to the generated app).

## liquid-glass: Liquid Glass Family

One project, four pages in the Apple liquid glass style:

- Glass landing `/`: dark colourful backdrop with soft glow ellipses,
  a floating glass capsule navigation, a large translucent hero card,
  glass buttons and stat chips, and three glass feature cards.
- Glass dashboard `/dashboard`: glass sidebar with an active pill, a
  glass top bar, four KPI cards, a bar chart card, a top products card
  and a recent orders strip.
- Glass shop `/shop`: four glass product cards with glowing product
  orbs and pill buy buttons over a colourful backdrop, plus a glass
  promo banner.
- Glass mobile `/mobile`: a 390 wide phone layout with a glass balance
  card, glass quick actions, a glass activity card and a floating
  glass capsule tab bar.

### How the glass works, honestly

The PNG and SVG renderer has no backdrop blur, so in renders the glass
is approximated with three ingredients the rasteriser does support:
translucent white fills, bright 1px strokes on the surface edge, and
coloured glow ellipses painted underneath the surfaces.

In generated web output the same nodes carry a `style_hint` property
with a real `backdrop-filter: blur(18px) saturate(160%)`, which
browsers apply over the live page backdrop. That is where the true
liquid glass blur appears; the static render is an honest preview
approximation, not the final web rendering.

## saas-dashboard: SaaS Dashboard

A dark SaaS admin panel in Persian, fully RTL. Two pages:

- Dashboard `/`: sidebar with an active item pill, a top bar with
  search and a report export button, four KPI cards (today revenue,
  new orders, new customers, conversion rate, each with a delta line),
  a weekly sales bar chart card, a top products card, and a latest
  orders table with status chips.
- Customers `/customers`: the same shell with a customers table
  (name, email, phone, total spent) and an active customers stat.

Database: `orders`, `customers` and `products` with Persian seed
rows and an order to customer relation. Flow: the export button
calls the generated orders API and shows a confirmation message.

## clinic: Mehr Clinic

A medical clinic site with online booking. Sample clinic name and
all people are invented sample data. Two pages:

- Home `/`: header, a soft teal hero, four service cards
  (dentistry, eye care, cardiology, skin and hair), four doctor
  cards with specialty and first free slot, a stats band and a
  footer.
- Appointment `/appointment`: a booking form (name, phone, doctor,
  date, time, notes) with a three step explainer card.

Database: `doctors`, `appointments` (related to doctors) and
`services`, with seed doctors and appointments. Flow: submitting
the form saves an appointment record, shows a confirmation and
returns home.

## restaurant: Taam Restaurant

A warm restaurant site with a table reservation form. Two pages:

- Home `/`: dark header and hero, four stat chips, three featured
  dish cards plus a chef suggestion banner, and a reservation form
  band (name, phone, guests, date).
- Menu `/menu`: the full menu in three categories (starters, mains,
  desserts and drinks), twelve dishes with descriptions and prices
  in Persian digits, in a classic price list layout.

Database: `menu_items` with the same twelve dishes as seed rows and
`reservations`. Flow: the reservation form saves a reservation and
shows a confirmation message.

## portfolio: Personal Portfolio

A light editorial portfolio with placeholder identity, ready to be
made yours. Two pages:

- Home `/`: minimal header, a hero with a photo placeholder circle
  and a big title, a six work grid, three numbered services and a
  dark call to action band.
- Contact `/contact`: a contact form (name, email, subject,
  message) with a contact info card.

Database: `works`, `messages` and `services`. Flow: the contact
form saves the message and shows a thank you note.

## Other templates

- `shop`: the starter plant shop with storefront, admin, seed
  products, orders and a checkout flow.
- `persian`: a small Persian RTL sample page that proves shaped
  rendering, including Persian digit prices.
- `blank`: one empty page with a title, for starting from scratch.
