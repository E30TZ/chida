<?php
/**
 * Fixture theme functions. The importer ignores this file; it is
 * here so the fixture looks like a real theme directory.
 */

function fixture_blog_setup() {
    add_theme_support( 'title-tag' );
    register_nav_menus( array( 'primary' => 'Primary' ) );
}
add_action( 'after_setup_theme', 'fixture_blog_setup' );
