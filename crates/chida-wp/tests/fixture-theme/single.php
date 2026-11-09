<?php get_header(); ?>
<main class="wrap">
  <article class="post-card">
    <h1 class="post-title"><?php the_title(); ?></h1>
    <p class="post-meta"><?php the_author(); ?> - <?php echo get_the_date(); ?></p>
    <?php the_post_thumbnail( 'large' ); ?>
    <div class="post-text"><?php the_content(); ?></div>
  </article>
</main>
<?php get_footer(); ?>
