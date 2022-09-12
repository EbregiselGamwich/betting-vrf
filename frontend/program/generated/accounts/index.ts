export * from './Game'
export * from './Stats'
export * from './UserAccount'
export * from './VrfResult'

import { Game } from './Game'
import { Stats } from './Stats'
import { UserAccount } from './UserAccount'
import { VrfResult } from './VrfResult'

export const accountProviders = { Game, Stats, UserAccount, VrfResult }
