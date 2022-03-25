#[cfg(test)]
mod test {
    use simplep2pgossip::p2pcache::{PeerCache, PeerList, PeerState};

    #[test]
    fn test_update_from_list() -> Result<(), String> {
        let mut cache = PeerCache::new(0);
        let initial_peers: PeerList = PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 0, available: true},
            PeerState { address: "b".to_string(), timestamp: 1, available: false},
            PeerState { address: "c".to_string(), timestamp: 2, available: true},
        ]};
        assert!(cache.update_from_list(&initial_peers)?);
        assert_eq!(cache.get_list()?, initial_peers);

        let extended_peers: PeerList = PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 0, available: true},
            PeerState { address: "b".to_string(), timestamp: 2, available: true},
            PeerState { address: "c".to_string(), timestamp: 3, available: false},
            PeerState { address: "d".to_string(), timestamp: 4, available: false},
        ]};
        assert!(cache.update_from_list(&extended_peers)?);
        assert_eq!(cache.get_list()?, extended_peers);

        let null_peers: PeerList = PeerList { peers: vec![]};
        assert!(!cache.update_from_list(&null_peers)?);
        assert_eq!(cache.get_list()?, extended_peers);

        let outdated_peers: PeerList = PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 1, available: true},
            PeerState { address: "b".to_string(), timestamp: 3, available: false},
            PeerState { address: "c".to_string(), timestamp: 2, available: true},
            PeerState { address: "d".to_string(), timestamp: 3, available: false},
        ]};
        assert!(cache.update_from_list(&outdated_peers)?);
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 1, available: true },
            PeerState { address: "b".to_string(), timestamp: 3, available: false },
            PeerState { address: "c".to_string(), timestamp: 3, available: false },
            PeerState { address: "d".to_string(), timestamp: 4, available: false }] });

        let refreshed_peers: PeerList = PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 2, available: true},
            PeerState { address: "b".to_string(), timestamp: 4, available: false},
            PeerState { address: "c".to_string(), timestamp: 5, available: true},
            PeerState { address: "d".to_string(), timestamp: 6, available: false},
        ]};
        assert!(!cache.update_from_list(&refreshed_peers)?);
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 2, available: true },
            PeerState { address: "b".to_string(), timestamp: 4, available: false },
            PeerState { address: "c".to_string(), timestamp: 5, available: true },
            PeerState { address: "d".to_string(), timestamp: 6, available: false }] });

        Ok(())
    }

    #[test]
    #[cfg(feature = "mock_time")]
    fn test_update_peer() -> Result<(), String> {
        let mut cache = PeerCache::new(0);
        cache.set_current_time(1);
        assert!(cache.update_peer("a", true)?);
        assert!(cache.update_peer("b", true)?);
        assert!(cache.update_peer("c", false)?);
        assert!(cache.update_peer("d", false)?);
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 1, available: true },
            PeerState { address: "b".to_string(), timestamp: 1, available: true },
            PeerState { address: "c".to_string(), timestamp: 1, available: false },
            PeerState { address: "d".to_string(), timestamp: 1, available: false }] });

        // do not update unavailable users
        cache.set_current_time(2);
        assert!(!cache.update_peer("a", true)?);
        assert!(!cache.update_peer("b", true)?);
        assert!(!cache.update_peer("c", false)?);
        assert!(!cache.update_peer("d", false)?);
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 2, available: true },
            PeerState { address: "b".to_string(), timestamp: 2, available: true },
            PeerState { address: "c".to_string(), timestamp: 1, available: false },
            PeerState { address: "d".to_string(), timestamp: 1, available: false }] });

        cache.set_current_time(3);
        assert!(cache.update_peer("a", false)?);
        assert!(cache.update_peer("b", false)?);
        assert!(cache.update_peer("c", true)?);
        assert!(cache.update_peer("d", true)?);
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 3, available: false },
            PeerState { address: "b".to_string(), timestamp: 3, available: false },
            PeerState { address: "c".to_string(), timestamp: 3, available: true },
            PeerState { address: "d".to_string(), timestamp: 3, available: true }] });

        Ok(())
    }

    #[test]
    #[cfg(feature = "mock_time")]
    fn test_cleanup_old_peers() -> Result<(), String> {
        let mut cache = PeerCache::new(5);
        let initial_peers: PeerList = PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 0, available: true},
            PeerState { address: "b".to_string(), timestamp: 1000, available: false},
            PeerState { address: "c".to_string(), timestamp: 2000, available: true},
            PeerState { address: "d".to_string(), timestamp: 3000, available: false},
        ]};
        assert!(cache.update_from_list(&initial_peers)?);
        assert_eq!(cache.cleanup_old_peers()?, ());
        assert_eq!(cache.get_list()?, initial_peers);

        cache.set_current_time(1001 + 5000);
        assert_eq!(cache.cleanup_old_peers()?, ());
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 0, available: true},
            PeerState { address: "c".to_string(), timestamp: 2000, available: true},
            PeerState { address: "d".to_string(), timestamp: 3000, available: false},] });

        cache.set_current_time(3001 + 5000);
        assert_eq!(cache.cleanup_old_peers()?, ());
        assert_eq!(cache.get_list()?, PeerList { peers: vec![
            PeerState { address: "a".to_string(), timestamp: 0, available: true},
            PeerState { address: "c".to_string(), timestamp: 2000, available: true}] });

        Ok(())
    }
}