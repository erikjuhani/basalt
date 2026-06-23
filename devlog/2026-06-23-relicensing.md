+++
title = "Relicensing Basalt."
date = "2026-06-23"
+++

`Basalt` is moving off `MIT`.

I do this project for fun, and I never minded people forking it or copying from
it. What I do want is for changes to the app to come back, or at least stay in
the open. `MIT` does not ask for that, so it was time for a change.

The app and the libraries now get different treatment.

`basalt-tui`, the application, is now `GPL-3.0-or-later`. It is copyleft, which
means if you distribute a modified version it has to stay open under the same
terms. That is the whole point.

`basalt-core` and `basalt-widgets`, the libraries, are now `Apache-2.0`. These
are meant to be reused, so they stay permissive. Yes, that means someone can
still take them closed. That is a deliberate trade I am happy to make for the
libraries.

I also added a small Contributor License Agreement. You keep your copyright and
agree to it by opening a pull request. It keeps things flexible if the licensing
needs to change again later.

Nothing changes for versions already out there. If you have an `MIT` build, it
stays `MIT`. The new terms apply from the next release onward.

Cheers!

— Erik
