// SPDX-License-Identifier: GPL-3.0-only
import { createRoot } from 'react-dom/client'
import { OriginalHost } from './OriginalHost'
import '../styles.css'
// Preserve the original cockpit's structural cascade after its visual styles.
import '../cockpit-panes.css'

createRoot(document.getElementById('root')!).render(<OriginalHost />)
