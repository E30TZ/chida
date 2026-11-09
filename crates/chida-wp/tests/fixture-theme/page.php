<?php get_header(); ?>
<main class="wrap">
  <h1 class="post-title"><?php the_title(); ?></h1>
  <div class="post-text"><?php the_content(); ?></div>
  <?php echo do_shortcode( '[fixture-gallery]' ); ?>
</main>
<?php get_footer(); ?>
